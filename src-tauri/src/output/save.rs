use chrono::Local;
use std::path::PathBuf;

pub fn generate_snap_path(save_dir: &PathBuf) -> PathBuf {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    save_dir.join(format!("sip-cc_snap_{timestamp}.png"))
}

pub fn generate_gif_path(save_dir: &PathBuf) -> PathBuf {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    save_dir.join(format!("sip-cc_gif_{timestamp}.gif"))
}

pub fn save_png(pixels: &[u8], width: u32, height: u32, path: &PathBuf) -> Result<(), String> {
    image::save_buffer(path, pixels, width, height, image::ColorType::Rgba8)
        .map_err(|e| format!("保存 PNG 失败: {e}"))
}
