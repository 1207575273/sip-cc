use serde::Deserialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::config::{ConfigManager, SelectionMode};
use crate::output::{clipboard, save};
use crate::snap::{capture, crop};
use crate::wal::logger::WalLogger;

#[derive(Debug, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 强制关闭指定 label 的窗口，无论成功与否都不 panic
fn force_close_window(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.set_always_on_top(false);
        let _ = win.set_fullscreen(false);
        let _ = win.close();
    }
}

/// 关闭所有可能存在的 overlay 窗口
pub fn close_all_overlays(app: &AppHandle) {
    for label in &["snap-selection", "gif-selection"] {
        force_close_window(app, label);
    }
}

#[tauri::command]
pub fn snap_region(app: AppHandle, region: Region) -> Result<String, String> {
    let wal = app.state::<WalLogger>();
    wal.info("SNAP", &format!(
        "开始截屏 | region={},{},{},{}", region.x, region.y, region.width, region.height
    ));

    // 第一步：先关闭 overlay 窗口，释放全屏控制权
    close_all_overlays(&app);

    // 等待窗口完全销毁（Windows 上需要一点时间）
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 第二步：截屏（此时屏幕上已无遮罩）
    let screen = capture::capture_primary_screen().map_err(|e| {
        wal.error("SNAP", &format!("截屏失败 | reason={e}"));
        e
    })?;

    let cropped = crop::crop_rgba(
        &screen.image, region.x as u32, region.y as u32, region.width, region.height,
    )?;

    let config_manager = app.state::<ConfigManager>();
    let save_dir = config_manager.get_save_directory();
    let path = save::generate_snap_path(&save_dir);

    save::save_png(cropped.as_raw(), cropped.width(), cropped.height(), &path)?;
    clipboard::copy_image(cropped.as_raw(), cropped.width() as usize, cropped.height() as usize)?;

    wal.info("SNAP", &format!("截屏完成 | path={}", path.to_string_lossy()));
    Ok(path.to_string_lossy().to_string())
}

/// Rust 端命令：前端调用此命令关闭 overlay（备用）
#[tauri::command]
pub fn close_overlay(app: AppHandle) -> Result<(), String> {
    close_all_overlays(&app);
    Ok(())
}

pub fn open_snap_overlay(app: &AppHandle) -> Result<(), String> {
    // 先清理可能残留的旧窗口
    close_all_overlays(app);

    let config_manager = app.state::<ConfigManager>();
    let mode = {
        let config = config_manager.config.lock().unwrap();
        config.selection_mode.clone()
    };

    let view = match mode {
        SelectionMode::Overlay => "snap-overlay",
        SelectionMode::DragRegion => "snap-drag",
    };

    WebviewWindowBuilder::new(
        app, "snap-selection",
        WebviewUrl::App(format!("index.html?view={view}").into()),
    )
    .title("sip-cc snap")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .fullscreen(true)
    .build()
    .map_err(|e| format!("打开截屏选区失败: {e}"))?;

    Ok(())
}
