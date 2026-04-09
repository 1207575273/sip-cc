use tauri::{AppHandle, Manager, State};

use crate::commands::gif_cmd::RecordingState;
use crate::config::{ConfigManager, HotkeyConfig};
use crate::hotkey::keybinding::Keybinding;
use crate::wal::logger::WalLogger;

/// 获取当前快捷键配置
#[tauri::command]
pub fn get_hotkeys(app: AppHandle) -> Result<serde_json::Value, String> {
    let config_manager = app.state::<ConfigManager>();
    let config = config_manager.config.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(&config.hotkeys).map_err(|e| e.to_string())
}

/// 设置快捷键配置并保存，同时触发热重载
#[tauri::command]
pub fn set_hotkeys(app: AppHandle, hotkeys: HotkeyConfig) -> Result<(), String> {
    // 校验：解析所有快捷键是否合法
    Keybinding::parse(&hotkeys.snap).map_err(|e| format!("截屏快捷键无效: {e}"))?;
    Keybinding::parse(&hotkeys.gif).map_err(|e| format!("录制快捷键无效: {e}"))?;
    Keybinding::parse(&hotkeys.force_quit).map_err(|e| format!("退出快捷键无效: {e}"))?;

    let config_manager = app.state::<ConfigManager>();
    {
        let mut config = config_manager.config.lock().map_err(|e| e.to_string())?;
        config.hotkeys = hotkeys.clone();
    }
    config_manager.save()?;

    // 热重载绑定
    crate::hotkey::register::reload_bindings(&hotkeys);

    let wal = app.state::<WalLogger>();
    wal.info(
        "CONFIG",
        &format!(
            "快捷键变更 | snap={} gif={} force_quit={}",
            hotkeys.snap, hotkeys.gif, hotkeys.force_quit
        ),
    );

    Ok(())
}

/// 重新加载快捷键（从当前内存配置重新解析绑定）
#[tauri::command]
pub fn reload_hotkeys(app: AppHandle) -> Result<(), String> {
    let config_manager = app.state::<ConfigManager>();
    let hotkeys = {
        let config = config_manager.config.lock().map_err(|e| e.to_string())?;
        config.hotkeys.clone()
    };
    crate::hotkey::register::reload_bindings(&hotkeys);
    Ok(())
}

/// 获取录制状态（快捷键设置界面需要知道是否在录制中）
#[tauri::command]
pub fn get_recording_status(recording: State<'_, RecordingState>) -> Result<bool, String> {
    let guard = recording.session.lock().map_err(|e| e.to_string())?;
    Ok(guard.is_some())
}
