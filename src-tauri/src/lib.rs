mod commands;
mod config;
mod gif;
mod hotkey;
mod output;
mod snap;
mod tray;
mod wal;
mod video;

use commands::gif_cmd::RecordingState;
use commands::video_cmd::VideoRecordingState;
use commands::long_snap_cmd::LongSnapManualState;
use commands::snap_cmd::{MacosOverlayCount, ScreenBuffer};
use config::ConfigManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::Manager;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use wal::logger::WalLogger;

/// 全局退出标志：true 表示用户主动退出，不拦截
static SHOULD_EXIT: AtomicBool = AtomicBool::new(false);

/// 供 tray.rs 调用：标记为主动退出
pub fn request_exit() {
    SHOULD_EXIT.store(true, Ordering::Relaxed);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ConfigManager::new())
        .manage(WalLogger::new())
        .manage(RecordingState {
            session: Mutex::new(None),
        })
        .manage(VideoRecordingState::new())
        .manage(MacosOverlayCount(Mutex::new(0)))
        .manage(ScreenBuffer {
            image: Mutex::new(None),
            bounds: Mutex::new(None),
            macos_monitors: Mutex::new(None),
        })
        .manage(LongSnapManualState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::snap_cmd::snap_region,
            commands::snap_cmd::close_overlay,
            commands::snap_cmd::set_overlay_mode,
            commands::snap_cmd::set_overlay_cursor_passthrough,
            commands::snap_cmd::get_screen_info,
            commands::gif_cmd::gif_start,
            commands::gif_cmd::gif_pause,
            commands::gif_cmd::gif_resume,
            commands::gif_cmd::gif_stop,
            commands::open_url,
            commands::close_window,
            commands::hotkey_cmd::get_hotkeys,
            commands::hotkey_cmd::set_hotkeys,
            commands::hotkey_cmd::reload_hotkeys,
            commands::hotkey_cmd::get_recording_status,
            commands::video_cmd::check_ffmpeg,
            commands::video_cmd::download_ffmpeg,
            commands::video_cmd::video_start,
            commands::video_cmd::video_pause,
            commands::video_cmd::video_resume,
            commands::video_cmd::video_stop,
            commands::long_snap_cmd::long_snap_supported,
            commands::long_snap_cmd::long_snap_start,
            commands::long_snap_cmd::long_snap_append_frame,
            commands::long_snap_cmd::long_snap_finish,
            commands::long_snap_cmd::long_snap_cancel,
        ])
        .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
            let wal = app.state::<WalLogger>();
            wal.info("APP", "应用启动");
            tray::create_tray(app.handle())?;
            hotkey::register::start_hotkey_listener(app.handle().clone());

            if cfg!(target_os = "macos") {
                let monitors: Vec<_> = app
                    .available_monitors()
                    .map_err(|e| format!("{e}"))?
                    .into_iter()
                    .collect();
                for (i, _) in monitors.iter().enumerate() {
                    let label = format!("overlay-{}", i);
                    let url = format!("index.html?view=snap-overlay&monitor={}", i);
                    WebviewWindowBuilder::new(
                        app,
                        &label,
                        WebviewUrl::App(url.into()),
                    )
                    .title("sip-cc overlay")
                    .transparent(true)
                    .decorations(false)
                    .visible(false)
                    .build()?;
                }
                if let Some(st) = app.try_state::<MacosOverlayCount>() {
                    if let Ok(mut n) = st.0.lock() {
                        *n = monitors.len();
                    }
                }
                wal.info(
                    "APP",
                    &format!("macOS: {} 个 overlay 窗口预创建完成", monitors.len()),
                );
            } else {
                WebviewWindowBuilder::new(
                    app,
                    "overlay",
                    WebviewUrl::App("index.html?view=snap-overlay".into()),
                )
                .title("sip-cc overlay")
                .transparent(true)
                .decorations(false)
                .visible(false)
                .build()?;
                wal.info("APP", "overlay 窗口预创建完成");
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building sip-cc")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                // 只有非主动退出时才拦截（窗口关闭不退出，保持托盘常驻）
                if !SHOULD_EXIT.load(Ordering::Relaxed) {
                    api.prevent_exit();
                }
            }
        });
}
