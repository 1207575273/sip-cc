pub mod gif_cmd;
pub mod hotkey_cmd;
pub mod snap_cmd;
pub mod video_cmd;

/// 用系统默认浏览器打开 URL
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("打开链接失败: {e}"))
}

/// 通用窗口关闭命令（从 Rust 端关闭指定 label 的窗口）
#[tauri::command]
pub fn close_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window(&label) {
        win.close().map_err(|e| format!("关闭窗口失败: {e}"))
    } else {
        Ok(()) // 窗口不存在也算成功
    }
}
