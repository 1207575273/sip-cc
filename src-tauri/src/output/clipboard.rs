use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::path::PathBuf;

/// 复制图片到剪贴板
/// 先写入 PNG 格式（macOS 兼容性好），失败则回退到 RGBA 原始格式
pub fn copy_image(pixels: &[u8], width: usize, height: usize) -> Result<(), String> {
    if let Ok(png_data) = encode_png(pixels, width as u32, height as u32) {
        if copy_png_to_clipboard(&png_data).is_ok() {
            return Ok(());
        }
    }

    let mut cb = Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
    let data = ImageData {
        width,
        height,
        bytes: Cow::Borrowed(pixels),
    };
    cb.set_image(data).map_err(|e| format!("复制图片失败: {e}"))
}

/// 复制文件到剪贴板（跨平台）
/// Windows: Win32 CF_HDROP（粘贴出来是文件，和资源管理器 Ctrl+C 一样）
/// macOS: osascript POSIX file（粘贴出来是文件）
/// Linux: 回退到路径文本
pub fn copy_file_to_clipboard(path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        return copy_file_windows(path);
    }

    #[cfg(target_os = "macos")]
    {
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
        eprintln!("osascript 失败: {}，回退到文本", String::from_utf8_lossy(&output.stderr));
    }

    // Linux 或回退：复制路径文本
    #[cfg(not(target_os = "windows"))]
    {
        let mut cb = Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
        cb.set_text(path.to_string_lossy().to_string())
            .map_err(|e| format!("复制路径失败: {e}"))
    }
}

/// Windows: 用 Win32 API 把文件放入剪贴板（CF_HDROP 格式）
/// 粘贴时等同于资源管理器里 Ctrl+C 复制文件
#[cfg(target_os = "windows")]
fn copy_file_windows(path: &PathBuf) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    // 将路径转为 UTF-16 宽字符，双 null 结尾
    let wide_path: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0)) // 路径 null 结尾
        .collect();

    // DROPFILES 结构体大小（20 字节）
    let dropfiles_size = 20u32;
    let total_size = dropfiles_size as usize + (wide_path.len() + 1) * 2; // +1 for extra null terminator

    unsafe {
        // Win32 API 声明
        #[link(name = "user32")]
        extern "system" {
            fn OpenClipboard(hwnd: *mut std::ffi::c_void) -> i32;
            fn CloseClipboard() -> i32;
            fn EmptyClipboard() -> i32;
            fn SetClipboardData(format: u32, hmem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }

        #[link(name = "kernel32")]
        extern "system" {
            fn GlobalAlloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void;
            fn GlobalLock(hmem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn GlobalUnlock(hmem: *mut std::ffi::c_void) -> i32;
        }

        const CF_HDROP: u32 = 15;
        const GMEM_MOVEABLE: u32 = 0x0002;
        const GMEM_ZEROINIT: u32 = 0x0040;

        // 分配全局内存
        let hmem = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_size);
        if hmem.is_null() {
            return Err("GlobalAlloc 失败".to_string());
        }

        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            return Err("GlobalLock 失败".to_string());
        }

        // 写入 DROPFILES 结构体
        // typedef struct _DROPFILES {
        //   DWORD pFiles;   // offset 0: 文件名列表的偏移量
        //   POINT pt;       // offset 4: 未使用
        //   BOOL  fNC;      // offset 12: 未使用
        //   BOOL  fWide;    // offset 16: TRUE = Unicode
        // }
        let dropfiles = ptr as *mut u8;
        // pFiles = dropfiles_size（文件名列表紧跟在结构体后面）
        *(dropfiles as *mut u32) = dropfiles_size;
        // fWide = 1 (TRUE, 使用 Unicode)
        *(dropfiles.add(16) as *mut u32) = 1;

        // 写入文件路径（UTF-16）
        let file_start = dropfiles.add(dropfiles_size as usize) as *mut u16;
        for (i, &ch) in wide_path.iter().enumerate() {
            *file_start.add(i) = ch;
        }
        // 额外的 null terminator（文件列表结束标志）
        *file_start.add(wide_path.len()) = 0;

        GlobalUnlock(hmem);

        // 打开剪贴板并设置数据
        if OpenClipboard(ptr::null_mut()) == 0 {
            return Err("OpenClipboard 失败".to_string());
        }
        EmptyClipboard();
        let result = SetClipboardData(CF_HDROP, hmem);
        CloseClipboard();

        if result.is_null() {
            return Err("SetClipboardData 失败".to_string());
        }

        Ok(())
    }
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
        use std::io::Write;
        let tmp = std::env::temp_dir().join("sip-cc-clipboard.png");
        let mut file = std::fs::File::create(&tmp)
            .map_err(|e| format!("创建临时文件失败: {e}"))?;
        file.write_all(png_data)
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        drop(file);

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
