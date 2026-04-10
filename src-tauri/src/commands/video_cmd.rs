use std::sync::Mutex;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::commands::snap_cmd::{close_floating_windows, get_scale_factor, hide_overlay, Region};
use crate::config::ConfigManager;
use crate::output::save;
use crate::video::encoder::{self, EncodeOptions};
use crate::video::ffmpeg_manager;
use crate::video::session::VideoRecordingSession;
use crate::wal::logger::WalLogger;

const RECORD_BAR_WIDTH: f64 = 300.0;
const RECORD_BAR_HEIGHT: f64 = 44.0;
const RECORD_BAR_OFFSET_Y: f64 = 6.0;
const OVERLAY_HIDE_DELAY_MS: u64 = 50;
const OVERLAY_SETTLE_DELAY_MS: u64 = 150;
const WINDOW_CLOSE_DELAY_MS: u64 = 50;
const INFO_WIDTH: f64 = 168.0;
const INFO_HEIGHT: f64 = 82.0;
const INFO_MARGIN: f64 = 6.0;

#[derive(Debug, Clone, Deserialize)]
pub struct VideoQuality {
    pub fps: Option<u16>,
    pub crf: Option<u8>,
    pub preset: Option<String>,
    pub label: Option<String>,
}

struct ActiveRecording {
    session: VideoRecordingSession,
    fps: u16,
    crf: u8,
    preset: String,
}

pub struct VideoRecordingState {
    inner: Mutex<Option<ActiveRecording>>,
}

impl VideoRecordingState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn check_ffmpeg() -> bool {
    ffmpeg_manager::is_ffmpeg_available()
}

#[tauri::command]
pub async fn download_ffmpeg(app: AppHandle) -> Result<String, String> {
    let wal = app.state::<WalLogger>();
    wal.info("VIDEO", "开始下载 FFmpeg");

    let path = ffmpeg_manager::download_ffmpeg(app.clone()).await?;

    wal.info(
        "VIDEO",
        &format!("FFmpeg 下载完成: {}", path.to_string_lossy()),
    );
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn video_start(
    app: AppHandle,
    region: Region,
    quality: Option<VideoQuality>,
    recording: State<'_, VideoRecordingState>,
) -> Result<(), String> {
    let wal = app.state::<WalLogger>();

    if !ffmpeg_manager::is_ffmpeg_available() {
        return Err("ffmpeg_not_found".to_string());
    }

    let scale = get_scale_factor(&app);
    let pad = 10.0_f64;
    let phys_x = ((region.x as f64 - pad).max(0.0) * scale) as u32;
    let phys_y = ((region.y as f64 - pad).max(0.0) * scale) as u32;
    let phys_w = ((region.width as f64 + pad * 2.0) * scale) as u32 & !1;
    let phys_h = ((region.height as f64 + pad * 2.0) * scale) as u32 & !1;

    let log_x = region.x;
    let log_y = region.y;
    let log_w = region.width;
    let log_h = region.height;

    let config_manager = app.state::<ConfigManager>();
    let quality_label = quality
        .as_ref()
        .and_then(|q| q.label.clone())
        .unwrap_or_default();
    let (fps, max_duration, crf, preset) = {
        let config = config_manager.config.lock().unwrap();
        let fps = quality.as_ref().and_then(|q| q.fps).unwrap_or(config.video_fps);
        let crf = quality.as_ref().and_then(|q| q.crf).unwrap_or(config.video_crf);
        let preset = quality
            .as_ref()
            .and_then(|q| q.preset.clone())
            .unwrap_or_else(|| config.video_preset.clone());
        (fps, config.video_max_duration_secs, crf, preset)
    };

    wal.info(
        "VIDEO",
        &format!(
            "开始录制 | logical={},{},{},{} | physical={},{},{},{} | scale={} | fps={} crf={} preset={}",
            region.x, region.y, region.width, region.height,
            phys_x, phys_y, phys_w, phys_h, scale, fps, crf, preset
        ),
    );

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let tmp_dir = ConfigManager::base_dir()
        .join("tmp")
        .join(format!("video_{timestamp}"));

    let session =
        VideoRecordingSession::start(phys_x, phys_y, phys_w, phys_h, fps, max_duration, tmp_dir)?;

    let mut guard = recording.inner.lock().map_err(|e| e.to_string())?;
    *guard = Some(ActiveRecording {
        session,
        fps,
        crf,
        preset,
    });

    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(OVERLAY_HIDE_DELAY_MS));
        hide_overlay(&app_clone);
        std::thread::sleep(std::time::Duration::from_millis(OVERLAY_SETTLE_DELAY_MS));

        let bar_w = RECORD_BAR_WIDTH;
        let bar_h = RECORD_BAR_HEIGHT;
        let bar_x = (log_x as f64 + log_w as f64) - bar_w;
        let bar_y = log_y as f64 + log_h as f64 + RECORD_BAR_OFFSET_Y;

        let _ = WebviewWindowBuilder::new(
            &app_clone,
            "record-bar",
            WebviewUrl::App("index.html?view=record-bar&mode=video".into()),
        )
        .title("sip-cc recording")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .inner_size(bar_w, bar_h)
        .position(bar_x.max(0.0), bar_y)
        .build();

        let region_url = format!(
            "index.html?view=record-region&x={}&y={}&w={}&h={}",
            log_x, log_y, log_w, log_h
        );
        let region_result: Result<tauri::WebviewWindow<tauri::Wry>, _> =
            WebviewWindowBuilder::new(
                &app_clone,
                "record-region",
                WebviewUrl::App(region_url.into()),
            )
            .title("sip-cc region")
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .position(log_x as f64, log_y as f64)
            .inner_size(log_w as f64, log_h as f64)
            .build();
        if let Ok(region_win) = region_result {
            let _ = region_win.set_ignore_cursor_events(true);
        }

        if (log_w as f64) > INFO_WIDTH + INFO_MARGIN * 2.0 {
            let info_x = log_x as f64 + log_w as f64 - INFO_WIDTH - INFO_MARGIN;
            let info_y = log_y as f64 + INFO_MARGIN;
            let info_url = format!(
                "index.html?view=record-info&mode=video&w={}&h={}&fps={}&quality={}",
                log_w, log_h, fps, quality_label
            );
            let info_result: Result<tauri::WebviewWindow<tauri::Wry>, _> =
                WebviewWindowBuilder::new(
                    &app_clone,
                    "record-info",
                    WebviewUrl::App(info_url.into()),
                )
                .title("sip-cc info")
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .inner_size(INFO_WIDTH, INFO_HEIGHT)
                .position(info_x, info_y)
                .build();
            if let Ok(info_win) = info_result {
                let _ = info_win.set_ignore_cursor_events(true);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn video_pause(
    app: AppHandle,
    recording: State<'_, VideoRecordingState>,
) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("VIDEO", "录制暂停");
    let guard = recording.inner.lock().map_err(|e| e.to_string())?;
    if let Some(active) = guard.as_ref() {
        active.session.pause();
    }
    let _ = app.emit("recording-paused", ());
    Ok(())
}

#[tauri::command]
pub fn video_resume(
    app: AppHandle,
    recording: State<'_, VideoRecordingState>,
) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("VIDEO", "录制继续");
    let guard = recording.inner.lock().map_err(|e| e.to_string())?;
    if let Some(active) = guard.as_ref() {
        active.session.resume();
    }
    let _ = app.emit("recording-resumed", ());
    Ok(())
}

#[tauri::command]
pub fn video_stop(
    app: AppHandle,
    recording: State<'_, VideoRecordingState>,
) -> Result<(), String> {
    let mut guard = recording.inner.lock().map_err(|e| e.to_string())?;
    let mut active = guard.take().ok_or("没有进行中的录制")?;
    let (tmp_dir, frame_count) = active.session.stop()?;

    if frame_count == 0 {
        encoder::cleanup_temp_frames(&tmp_dir);
        return Err("没有捕获到任何帧".to_string());
    }

    let fps = active.fps;
    let crf = active.crf;
    let preset = active.preset;

    let config_manager = app.state::<ConfigManager>();
    let save_dir = config_manager.get_save_directory();
    let output_path = save::generate_video_path(&save_dir);

    std::thread::spawn(move || {
        let result = encoder::encode_video_with_progress(
            &tmp_dir,
            &output_path,
            &EncodeOptions { fps, crf, preset },
            frame_count,
            |current, total| {
                let _ = app.emit("video-encoding-progress", (current, total));
            },
        );

        encoder::cleanup_temp_frames(&tmp_dir);

        match result {
            Ok(path) => {
                let _ = crate::output::clipboard::copy_file_to_clipboard(&path);
                let wal = app.state::<WalLogger>();
                wal.info(
                    "VIDEO",
                    &format!("录制完成 | frames={} | path={}", frame_count, path.to_string_lossy()),
                );
                let _ = app.emit("video-encoding-done", path.to_string_lossy().to_string());
            }
            Err(e) => {
                let _ = app.emit("video-encoding-error", e);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(WINDOW_CLOSE_DELAY_MS));
        close_floating_windows(&app);
    });

    Ok(())
}

pub fn open_video_overlay(app: &AppHandle) -> Result<(), String> {
    if !ffmpeg_manager::is_ffmpeg_available() {
        open_ffmpeg_download_window(app);
        return Ok(());
    }

    let wal = app.state::<WalLogger>();
    wal.info("VIDEO", "F5/菜单触发，打开视频选区");

    crate::commands::snap_cmd::open_overlay_window(app)?;

    std::thread::sleep(std::time::Duration::from_millis(OVERLAY_HIDE_DELAY_MS));
    let _ = app.emit("overlay-mode", "video-overlay");

    Ok(())
}

fn open_ffmpeg_download_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("ffmpeg-download") {
        let _ = win.set_focus();
        return;
    }

    let _ = WebviewWindowBuilder::new(
        app,
        "ffmpeg-download",
        WebviewUrl::App("index.html?view=ffmpeg-download".into()),
    )
    .title("下载 FFmpeg")
    .inner_size(420.0, 260.0)
    .resizable(false)
    .center()
    .build();
}
