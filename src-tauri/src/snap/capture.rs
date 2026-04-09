use image::RgbaImage;
use xcap::Monitor;

pub struct ScreenCapture {
    pub image: RgbaImage,
}

/// 获取主显示器
fn get_primary_monitor() -> Result<Monitor, String> {
    Monitor::all()
        .map_err(|e| format!("获取显示器列表失败: {e}"))?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or_else(|| "未找到主显示器".to_string())
}

/// 截取主显示器全屏
pub fn capture_primary_screen() -> Result<ScreenCapture, String> {
    let monitor = get_primary_monitor()?;
    let image = monitor
        .capture_image()
        .map_err(|e| format!("屏幕截取失败: {e}"))?;
    Ok(ScreenCapture { image })
}

/// 截取主显示器指定区域（比截全屏再裁剪性能高很多）
pub fn capture_region(x: u32, y: u32, width: u32, height: u32) -> Result<RgbaImage, String> {
    let monitor = get_primary_monitor()?;
    monitor
        .capture_region(x, y, width, height)
        .map_err(|e| format!("区域截取失败: {e}"))
}
