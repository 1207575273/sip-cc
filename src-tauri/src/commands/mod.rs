pub mod gif_cmd;
pub mod hotkey_cmd;
pub mod snap_cmd;

/// 用系统默认浏览器打开 URL
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("打开链接失败: {e}"))
}
