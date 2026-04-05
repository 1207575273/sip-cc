use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::commands::snap_cmd::{close_all_overlays, get_scale_factor, Region};
use crate::config::ConfigManager;
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

    let scale = get_scale_factor();
    let phys_x = (region.x as f64 * scale) as u32;
    let phys_y = (region.y as f64 * scale) as u32;
    let phys_w = (region.width as f64 * scale) as u32;
    let phys_h = (region.height as f64 * scale) as u32;

    // 保存逻辑坐标，用于创建指示框窗口
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

    // 独立线程：关闭 overlay → 打开录制控制条 + 区域指示框
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        close_all_overlays(&app_clone);
        std::thread::sleep(std::time::Duration::from_millis(200));

        // 录制控制条（顶部居中小窗口）
        let _ = WebviewWindowBuilder::new(
            &app_clone, "record-bar",
            WebviewUrl::App("index.html?view=record-bar".into()),
        )
        .title("sip-cc recording")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .inner_size(280.0, 36.0)
        .center()
        .build();

        // 录制区域指示框（精确覆盖录制区域，鼠标穿透）
        let region_url = format!(
            "index.html?view=record-region&x={}&y={}&w={}&h={}",
            log_x, log_y, log_w, log_h
        );
        if let Ok(region_win) = WebviewWindowBuilder::new(
            &app_clone, "record-region",
            WebviewUrl::App(region_url.into()),
        )
        .title("sip-cc region")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .position(log_x as f64, log_y as f64)
        .inner_size(log_w as f64, log_h as f64)
        .build()
        {
            // 设置鼠标穿透，不影响用户操作
            let _ = region_win.set_ignore_cursor_events(true);
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

    // 独立线程关闭录制控制条和区域指示框
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some(win) = app_clone.get_webview_window("record-bar") {
            let _ = win.close();
        }
        if let Some(win) = app_clone.get_webview_window("record-region") {
            let _ = win.close();
        }
    });

    let mut cb = arboard::Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
    cb.set_text(path.to_string_lossy().to_string())
        .map_err(|e| format!("复制路径失败: {e}"))?;

    wal.info("GIF", &format!("录制停止 | path={}", path.to_string_lossy()));
    Ok(path.to_string_lossy().to_string())
}

pub fn open_gif_overlay(app: &AppHandle) -> Result<(), String> {
    close_all_overlays(app);

    WebviewWindowBuilder::new(
        app, "gif-selection",
        WebviewUrl::App("index.html?view=gif-overlay".into()),
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
