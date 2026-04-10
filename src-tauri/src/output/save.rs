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

pub fn generate_video_path(save_dir: &PathBuf) -> PathBuf {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    save_dir.join(format!("sip-cc_video_{timestamp}.mp4"))
}

pub fn save_png(pixels: &[u8], width: u32, height: u32, path: &PathBuf) -> Result<(), String> {
    image::save_buffer(path, pixels, width, height, image::ColorType::Rgba8)
        .map_err(|e| format!("保存 PNG 失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_generate_snap_path_with_png_extension() {
        let dir = PathBuf::from("/tmp");
        let path = generate_snap_path(&dir);
        assert!(path.to_string_lossy().contains("sip-cc_snap_"));
        assert!(path.to_string_lossy().ends_with(".png"));
        assert!(path.starts_with("/tmp"));
    }

    #[test]
    fn should_generate_gif_path_with_gif_extension() {
        let dir = PathBuf::from("/tmp");
        let path = generate_gif_path(&dir);
        assert!(path.to_string_lossy().contains("sip-cc_gif_"));
        assert!(path.to_string_lossy().ends_with(".gif"));
    }

    #[test]
    fn should_generate_video_path_with_mp4_extension() {
        let dir = PathBuf::from("/tmp");
        let path = generate_video_path(&dir);
        assert!(path.to_string_lossy().contains("sip-cc_video_"));
        assert!(path.to_string_lossy().ends_with(".mp4"));
    }

    #[test]
    fn should_generate_different_names_for_snap_and_gif() {
        let dir = PathBuf::from("/tmp");
        let snap = generate_snap_path(&dir);
        let gif = generate_gif_path(&dir);
        assert_ne!(snap, gif);
    }

    #[test]
    fn should_save_png_to_temp_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("sip-cc-test-output.png");
        // 2x2 红色像素
        let pixels: Vec<u8> = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255];
        let result = save_png(&pixels, 2, 2, &path);
        assert!(result.is_ok());
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }
}
