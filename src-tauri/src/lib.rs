mod config;
mod output;
mod snap;
mod tray;
mod wal;

use config::ConfigManager;
use tauri::Manager;
use wal::logger::WalLogger;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ConfigManager::new())
        .manage(WalLogger::new())
        .setup(|app| {
            let wal = app.state::<WalLogger>();
            wal.info("APP", "应用启动");
            tray::create_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running sip-cc");
}
