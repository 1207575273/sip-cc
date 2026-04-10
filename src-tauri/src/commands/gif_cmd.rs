use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::commands::snap_cmd::{close_floating_windows, get_scale_factor, hide_overlay, Region};
use crate::config::ConfigManager;
use crate::gif::frame::RecordingSession;
use crate::output::save;
use crate::wal::logger::WalLogger;

const RECORD_BAR_WIDTH: f64 = 300.0;
const RECORD_BAR_HEIGHT: f64 = 44.0;
const RECORD_BAR_OFFSET_Y: f64 = 6.0;
const WINDOW_CLOSE_DELAY_MS: u64 = 50;
const OVERLAY_HIDE_DELAY_MS: u64 = 50;
const OVERLAY_SETTLE_DELAY_MS: u64 = 150;
const INFO_WIDTH: f64 = 168.0;
const INFO_HEIGHT: f64 = 82.0;
const INFO_MARGIN: f64 = 6.0;

pub struct RecordingState {
    pub session: Mutex<Option<RecordingSession>>,
}

#[tauri::command]
pub fn gif_start(
    app: AppHandle, region: Region, recording: State<'_, RecordingState>,
) -> Result<(), String> {
    let wal = app.state::<WalLogger>();

    let scale = get_scale_factor(&app);
    let pad = 10.0_f64;
    let phys_x = ((region.x as f64 - pad).max(0.0) * scale) as u32;
    let phys_y = ((region.y as f64 - pad).max(0.0) * scale) as u32;
    let phys_w = ((region.width as f64 + pad * 2.0) * scale) as u32;
    let phys_h = ((region.height as f64 + pad * 2.0) * scale) as u32;

    let log_x = region.x;
    let log_y = region.y;
    let log_w = region.width;
    let log_h = region.height;

    wal.info("GIF", &format!(
        "开始录制 | logical={},{},{},{} | physical={},{},{},{} | scale={}",
        region.x, region.y, region.width, region.height,
        phys_x, phys_y, phys_w, phys_h, scale
    ));

    let config_manager = app.state::<ConfigManager>();
    let (fps, max_duration) = {
        let config = config_manager.config.lock().unwrap();
        (config.gif_fps, config.gif_max_duration_secs)
    };
    let save_dir = config_manager.get_save_directory();
    let output_path = save::generate_gif_path(&save_dir);

    let session = RecordingSession::start(
        phys_x, phys_y, phys_w, phys_h,
        fps, max_duration, output_path,
    )?;

    let mut guard = recording.session.lock().map_err(|e| e.to_string())?;
    *guard = Some(session);

    // 独立线程：隐藏 overlay → 打开录制控制条 + 区域指示框
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(OVERLAY_HIDE_DELAY_MS));
        hide_overlay(&app_clone);
        std::thread::sleep(std::time::Duration::from_millis(OVERLAY_SETTLE_DELAY_MS));

        // 录制控制条——定位在录制区域右下角
        let bar_w = RECORD_BAR_WIDTH;
        let bar_h = RECORD_BAR_HEIGHT;
        let bar_x = (log_x as f64 + log_w as f64) - bar_w;
        let bar_y = log_y as f64 + log_h as f64 + RECORD_BAR_OFFSET_Y;
        let _ = WebviewWindowBuilder::new(
            &app_clone, "record-bar",
            WebviewUrl::App("index.html?view=record-bar".into()),
        )
        .title("sip-cc recording")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .inner_size(bar_w, bar_h)
        .position(bar_x.max(0.0), bar_y)
        .build();

        // 录制区域指示框（鼠标穿透）
        let region_url = format!(
            "index.html?view=record-region&x={}&y={}&w={}&h={}",
            log_x, log_y, log_w, log_h
        );
        let region_result: Result<tauri::WebviewWindow<tauri::Wry>, _> = WebviewWindowBuilder::new(
            &app_clone, "record-region",
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
                "index.html?view=record-info&mode=gif&w={}&h={}&fps={}",
                log_w, log_h, fps
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
pub fn gif_pause(app: AppHandle, recording: State<'_, RecordingState>) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("GIF", "录制暂停");
    let guard = recording.session.lock().map_err(|e| e.to_string())?;
    if let Some(session) = guard.as_ref() { session.pause(); }
    let _ = app.emit("recording-paused", ());
    Ok(())
}

#[tauri::command]
pub fn gif_resume(app: AppHandle, recording: State<'_, RecordingState>) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("GIF", "录制继续");
    let guard = recording.session.lock().map_err(|e| e.to_string())?;
    if let Some(session) = guard.as_ref() { session.resume(); }
    let _ = app.emit("recording-resumed", ());
    Ok(())
}

#[tauri::command]
pub fn gif_stop(app: AppHandle, recording: State<'_, RecordingState>) -> Result<String, String> {
    let wal = app.state::<WalLogger>();
    let mut guard = recording.session.lock().map_err(|e| e.to_string())?;
    let mut session = guard.take().ok_or("没有进行中的录制")?;
    let path = session.stop()?;

    // 独立线程关闭录制相关窗口
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(WINDOW_CLOSE_DELAY_MS));
        close_floating_windows(&app_clone);
    });

    // 复制 GIF 文件到剪贴板（macOS: 文件引用，Windows: 路径文本）
    crate::output::clipboard::copy_file_to_clipboard(&path)?;

    wal.info("GIF", &format!("录制停止 | path={}", path.to_string_lossy()));
    Ok(path.to_string_lossy().to_string())
}

/// F3 触发：显示 overlay（GIF 模式），复用 snap_cmd::show_overlay 的平台适配逻辑
pub fn open_gif_overlay(app: &AppHandle) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("GIF", "F3/菜单触发，打开 GIF 选区");

    // 复用 show_overlay 处理 macOS/Windows 差异
    crate::commands::snap_cmd::open_overlay_window(app)?;

    std::thread::sleep(std::time::Duration::from_millis(OVERLAY_HIDE_DELAY_MS));
    let _ = app.emit("overlay-mode", "gif-overlay");

    Ok(())
}
