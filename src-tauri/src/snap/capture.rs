use image::RgbaImage;
use xcap::Monitor;

pub struct ScreenCapture {
    pub image: RgbaImage,
    pub width: u32,
    pub height: u32,
}

pub fn capture_primary_screen() -> Result<ScreenCapture, String> {
    let monitor = Monitor::all()
        .map_err(|e| format!("获取显示器列表失败: {e}"))?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or_else(|| "未找到主显示器".to_string())?;

    let image = monitor
        .capture_image()
        .map_err(|e| format!("屏幕截取失败: {e}"))?;

    let width = image.width();
    let height = image.height();

    Ok(ScreenCapture { image, width, height })
}
