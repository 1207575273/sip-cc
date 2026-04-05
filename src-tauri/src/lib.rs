mod commands;
mod config;
mod gif;
mod hotkey;
mod output;
mod snap;
mod tray;
mod wal;

use commands::gif_cmd::RecordingState;
use commands::snap_cmd::ScreenBuffer;
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
        .manage(ScreenBuffer {
            image: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::snap_cmd::snap_region,
            commands::snap_cmd::close_overlay,
            commands::snap_cmd::set_overlay_mode,
            commands::gif_cmd::gif_start,
            commands::gif_cmd::gif_pause,
            commands::gif_cmd::gif_resume,
            commands::gif_cmd::gif_stop,
        ])
        .setup(|app| {
            let wal = app.state::<WalLogger>();
            wal.info("APP", "应用启动");
            tray::create_tray(app.handle())?;
            hotkey::register::start_hotkey_listener(app.handle().clone());

            let _overlay = WebviewWindowBuilder::new(
                app, "overlay",
                WebviewUrl::App("index.html?view=snap-overlay".into()),
            )
            .title("sip-cc overlay")
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .fullscreen(true)
            .visible(false)
            .build()?;

            wal.info("APP", "overlay 窗口预创建完成");
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
