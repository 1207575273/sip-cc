use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::path::PathBuf;

/// 复制图片到剪贴板
/// 先写入 PNG 格式（macOS 兼容性好），失败则回退到 RGBA 原始格式
pub fn copy_image(pixels: &[u8], width: usize, height: usize) -> Result<(), String> {
    // 尝试用 PNG 格式写入（macOS 上 RGBA 格式不被微信等应用识别）
    if let Ok(png_data) = encode_png(pixels, width as u32, height as u32) {
        if copy_png_to_clipboard(&png_data).is_ok() {
            return Ok(());
        }
    }

    // 回退到 RGBA 格式（Windows 上这个更通用）
    let mut cb = Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
    let data = ImageData {
        width,
        height,
        bytes: Cow::Borrowed(pixels),
    };
    cb.set_image(data).map_err(|e| format!("复制图片失败: {e}"))
}

/// 复制文件到剪贴板（跨平台）
/// macOS: 通过 osascript 写入文件引用，粘贴时是文件本身
/// Windows/Linux: 复制文件路径文本
pub fn copy_file_to_clipboard(path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // macOS 用 osascript 把文件写入剪贴板（粘贴出来是文件，不是路径文本）
        let path_str = path.to_string_lossy();
        let script = format!(
            "set the clipboard to (POSIX file \"{}\")",
            path_str
        );
        let output = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("osascript 执行失败: {e}"))?;

        if output.status.success() {
            return Ok(());
        }
        // osascript 失败，回退到文本
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("osascript 失败: {stderr}，回退到文本复制");
    }

    // Windows/Linux 或 macOS 回退：复制路径文本
    let mut cb = Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
    cb.set_text(path.to_string_lossy().to_string())
        .map_err(|e| format!("复制路径失败: {e}"))
}

/// 将 RGBA 像素编码为 PNG 字节
fn encode_png(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder
        .write_image(pixels, width, height, image::ColorType::Rgba8.into())
        .map_err(|e| format!("PNG 编码失败: {e}"))?;
    Ok(buf)
}

/// 用平台原生方式写入 PNG 到剪贴板
fn copy_png_to_clipboard(png_data: &[u8]) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // macOS: 通过 pbcopy 或 osascript 不方便写二进制，用 arboard 的 image 方式
        // 但先解码回 RGBA 让 arboard 处理格式转换
        // 实际上 arboard 在 macOS 上内部会转为 NSImage，PNG 和 RGBA 效果一样
        // 所以 macOS 上我们直接用临时文件 + osascript 写入 PNG
        use std::io::Write;
        let tmp = std::env::temp_dir().join("sip-cc-clipboard.png");
        let mut file = std::fs::File::create(&tmp)
            .map_err(|e| format!("创建临时文件失败: {e}"))?;
        file.write_all(png_data)
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        drop(file);

        // 用 osascript 把 PNG 图片写入剪贴板
        let script = format!(
            "set the clipboard to (read (POSIX file \"{}\") as «class PNGf»)",
            tmp.to_string_lossy()
        );
        let output = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("osascript 执行失败: {e}"))?;

        let _ = std::fs::remove_file(&tmp);

        if output.status.success() {
            return Ok(());
        }
        return Err(format!("osascript 失败: {}", String::from_utf8_lossy(&output.stderr)));
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Windows/Linux: arboard 的 set_image 已经能正确处理
        // 解码 PNG 回 RGBA 再用 arboard
        let img = image::load_from_memory(png_data)
            .map_err(|e| format!("PNG 解码失败: {e}"))?;
        let rgba = img.to_rgba8();
        let mut cb = Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
        let data = ImageData {
            width: rgba.width() as usize,
            height: rgba.height() as usize,
            bytes: Cow::Borrowed(rgba.as_raw()),
        };
        cb.set_image(data).map_err(|e| format!("复制图片失败: {e}"))
    }
}
