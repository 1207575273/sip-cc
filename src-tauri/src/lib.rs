mod commands;
mod config;
mod gif;
mod hotkey;
mod output;
mod snap;
mod tray;
mod wal;

use commands::gif_cmd::RecordingState;
use config::ConfigManager;
use std::sync::Mutex;
use tauri::Manager;
use wal::logger::WalLogger;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ConfigManager::new())
        .manage(WalLogger::new())
        .manage(RecordingState {
            session: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::snap_cmd::snap_region,
            commands::snap_cmd::close_overlay,
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
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building sip-cc")
        .run(|_app, event| {
            // 所有窗口关闭时不退出应用，保持托盘常驻
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
