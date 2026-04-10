use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::ffmpeg_manager::get_ffmpeg_path;

pub struct EncodeOptions {
    pub fps: u16,
    pub crf: u8,
    pub preset: String,
}

pub fn encode_video_with_progress<F>(
    tmp_dir: &PathBuf,
    output_path: &PathBuf,
    options: &EncodeOptions,
    total_frames: u32,
    on_progress: F,
) -> Result<PathBuf, String>
where
    F: Fn(u32, u32),
{
    let ffmpeg = get_ffmpeg_path();
    let input_pattern = tmp_dir.join("frame_%06d.png");
    let input_str = input_pattern.to_string_lossy().to_string();
    let output_str = output_path.to_string_lossy().to_string();

    let mut child = Command::new(&ffmpeg)
        .args([
            "-y",
            "-framerate", &options.fps.to_string(),
            "-start_number", "1",
            "-i", &input_str,
            "-c:v", "libx264",
            "-crf", &options.crf.to_string(),
            "-preset", &options.preset,
            "-pix_fmt", "yuv420p",
            "-movflags", "+faststart",
            "-threads", "0",
            "-vsync", "cfr",
            "-progress", "pipe:2",
            &output_str,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("FFmpeg 启动失败: {e}"))?;

    let stderr = child.stderr.take();
    let mut last_frame: u32 = 0;

    if let Some(pipe) = stderr {
        let reader = BufReader::new(pipe);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(val) = line.strip_prefix("frame=") {
                if let Ok(f) = val.trim().parse::<u32>() {
                    last_frame = f;
                    on_progress(f, total_frames);
                }
            }
        }
    }

    let status = child.wait().map_err(|e| format!("FFmpeg 等待失败: {e}"))?;
    if !status.success() {
        return Err(format!("FFmpeg 编码失败 (exit {})", status.code().unwrap_or(-1)));
    }

    if last_frame > 0 {
        on_progress(total_frames, total_frames);
    }

    validate_output(output_path)?;

    Ok(output_path.clone())
}

const MIN_VALID_MP4_SIZE: u64 = 1024;

fn validate_output(path: &PathBuf) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|_| "输出文件不存在".to_string())?;
    if meta.len() < MIN_VALID_MP4_SIZE {
        let _ = std::fs::remove_file(path);
        return Err(format!(
            "输出文件异常 ({}B)，可能编码失败，已删除",
            meta.len()
        ));
    }
    Ok(())
}

pub fn cleanup_temp_frames(tmp_dir: &PathBuf) {
    if tmp_dir.exists() {
        let _ = std::fs::remove_dir_all(tmp_dir);
    }
}
