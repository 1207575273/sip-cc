use serde::Deserialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use xcap::Monitor;

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

fn force_close_window(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.set_always_on_top(false);
        let _ = win.set_fullscreen(false);
        let _ = win.close();
    }
}

pub fn close_all_overlays(app: &AppHandle) {
    for label in &["snap-selection", "gif-selection"] {
        force_close_window(app, label);
    }
}

/// 获取主显示器的 DPI 缩放因子
pub fn get_scale_factor() -> f64 {
    Monitor::all()
        .ok()
        .and_then(|monitors| {
            monitors.into_iter()
                .find(|m| m.is_primary().unwrap_or(false))
        })
        .and_then(|m| m.scale_factor().ok())
        .unwrap_or(1.0) as f64
}

#[tauri::command]
pub fn snap_region(app: AppHandle, region: Region) -> Result<String, String> {
    let wal = app.state::<WalLogger>();

    // 获取 DPI 缩放因子，将前端逻辑像素转为屏幕物理像素
    let scale = get_scale_factor();
    let phys_x = (region.x as f64 * scale) as u32;
    let phys_y = (region.y as f64 * scale) as u32;
    let phys_w = (region.width as f64 * scale) as u32;
    let phys_h = (region.height as f64 * scale) as u32;

    wal.info("SNAP", &format!(
        "开始截屏 | logical={},{},{},{} | physical={},{},{},{} | scale={}",
        region.x, region.y, region.width, region.height,
        phys_x, phys_y, phys_w, phys_h, scale
    ));

    // 先关闭 overlay 窗口
    close_all_overlays(&app);
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 截取全屏
    let screen = capture::capture_primary_screen().map_err(|e| {
        wal.error("SNAP", &format!("截屏失败 | reason={e}"));
        e
    })?;

    // 用物理像素坐标裁剪
    let cropped = crop::crop_rgba(&screen.image, phys_x, phys_y, phys_w, phys_h)?;

    let config_manager = app.state::<ConfigManager>();
    let save_dir = config_manager.get_save_directory();
    let path = save::generate_snap_path(&save_dir);

    save::save_png(cropped.as_raw(), cropped.width(), cropped.height(), &path)?;
    clipboard::copy_image(cropped.as_raw(), cropped.width() as usize, cropped.height() as usize)?;

    wal.info("SNAP", &format!("截屏完成 | path={} | size={}x{}",
        path.to_string_lossy(), cropped.width(), cropped.height()));
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn close_overlay(app: AppHandle) -> Result<(), String> {
    close_all_overlays(&app);
    Ok(())
}

pub fn open_snap_overlay(app: &AppHandle) -> Result<(), String> {
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
