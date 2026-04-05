use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::commands::snap_cmd::{close_all_overlays, Region};
use crate::config::{ConfigManager, SelectionMode};
use crate::gif::frame::RecordingSession;
use crate::output::save;
use crate::wal::logger::WalLogger;

pub struct RecordingState {
    pub session: Mutex<Option<RecordingSession>>,
}

#[tauri::command]
pub fn gif_start(
    app: AppHandle, region: Region, recording: State<'_, RecordingState>,
) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("GIF", &format!(
        "开始录制 | region={},{},{},{}", region.x, region.y, region.width, region.height
    ));

    // 先关闭选区 overlay，释放全屏控制权
    close_all_overlays(&app);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let config_manager = app.state::<ConfigManager>();
    let (fps, max_duration) = {
        let config = config_manager.config.lock().unwrap();
        (config.gif_fps, config.gif_max_duration_secs)
    };
    let save_dir = config_manager.get_save_directory();
    let output_path = save::generate_gif_path(&save_dir);

    let session = RecordingSession::start(
        region.x as u32, region.y as u32, region.width, region.height,
        fps, max_duration, output_path,
    )?;

    let mut guard = recording.session.lock().map_err(|e| e.to_string())?;
    *guard = Some(session);

    // 打开录制控制条（小窗口，非全屏）
    WebviewWindowBuilder::new(
        &app, "record-bar",
        WebviewUrl::App("index.html?view=record-bar".into()),
    )
    .title("sip-cc recording")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .inner_size(320.0, 48.0)
    .center()
    .build()
    .map_err(|e| format!("打开录制控制条失败: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn gif_pause(app: AppHandle, recording: State<'_, RecordingState>) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("GIF", "录制暂停");
    let guard = recording.session.lock().map_err(|e| e.to_string())?;
    if let Some(session) = guard.as_ref() { session.pause(); }
    Ok(())
}

#[tauri::command]
pub fn gif_resume(app: AppHandle, recording: State<'_, RecordingState>) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("GIF", "录制继续");
    let guard = recording.session.lock().map_err(|e| e.to_string())?;
    if let Some(session) = guard.as_ref() { session.resume(); }
    Ok(())
}

#[tauri::command]
pub fn gif_stop(app: AppHandle, recording: State<'_, RecordingState>) -> Result<String, String> {
    let wal = app.state::<WalLogger>();
    let mut guard = recording.session.lock().map_err(|e| e.to_string())?;
    let mut session = guard.take().ok_or("没有进行中的录制")?;
    let path = session.stop()?;

    // 关闭录制控制条
    if let Some(win) = app.get_webview_window("record-bar") {
        let _ = win.close();
    }

    let mut cb = arboard::Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
    cb.set_text(path.to_string_lossy().to_string())
        .map_err(|e| format!("复制路径失败: {e}"))?;

    wal.info("GIF", &format!("录制停止 | path={}", path.to_string_lossy()));
    Ok(path.to_string_lossy().to_string())
}

pub fn open_gif_overlay(app: &AppHandle) -> Result<(), String> {
    close_all_overlays(app);

    let config_manager = app.state::<ConfigManager>();
    let mode = {
        let config = config_manager.config.lock().unwrap();
        config.selection_mode.clone()
    };

    let view = match mode {
        SelectionMode::Overlay => "gif-overlay",
        SelectionMode::DragRegion => "gif-drag",
    };

    WebviewWindowBuilder::new(
        app, "gif-selection",
        WebviewUrl::App(format!("index.html?view={view}").into()),
    )
    .title("sip-cc gif")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .fullscreen(true)
    .build()
    .map_err(|e| format!("打开 GIF 选区失败: {e}"))?;

    Ok(())
}
