use image::RgbaImage;
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use xcap::Monitor;

use crate::config::ConfigManager;
use crate::output::{clipboard, save};
use crate::snap::crop;
use crate::wal::logger::WalLogger;

#[derive(Debug, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct ScreenBuffer {
    pub image: Mutex<Option<RgbaImage>>,
}

pub fn get_scale_factor() -> f64 {
    Monitor::all()
        .ok()
        .and_then(|monitors| monitors.into_iter().find(|m| m.is_primary().unwrap_or(false)))
        .and_then(|m| m.scale_factor().ok())
        .unwrap_or(1.0) as f64
}

/// 显示 overlay 窗口（复用预创建的窗口）
fn show_overlay(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.set_fullscreen(true);
        let _ = win.set_always_on_top(true);
        win.show().map_err(|e| format!("显示 overlay 失败: {e}"))?;
        let _ = win.set_focus();
        Ok(())
    } else {
        Err("overlay 窗口不存在".to_string())
    }
}

/// 隐藏 overlay 窗口（不销毁，节省内存）
pub fn hide_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.set_fullscreen(false);
        let _ = win.set_always_on_top(false);
        let _ = win.hide();
    }
}

/// 关闭所有可能存在的浮动窗口（record-bar、record-region）
pub fn close_floating_windows(app: &AppHandle) {
    for label in &["record-bar", "record-region"] {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.close();
        }
    }
}

/// 前端调用：通知 overlay 切换到指定模式
#[tauri::command]
pub fn set_overlay_mode(app: AppHandle, mode: String) -> Result<(), String> {
    app.emit("overlay-mode", mode).map_err(|e| e.to_string())
}

/// 前端调用：关闭/隐藏 overlay
#[tauri::command]
pub fn close_overlay(app: AppHandle) -> Result<(), String> {
    hide_overlay(&app);
    Ok(())
}

/// 截屏命令：从预截的全屏图中裁剪
#[tauri::command]
pub fn snap_region(app: AppHandle, region: Region) -> Result<String, String> {
    let wal = app.state::<WalLogger>();

    let scale = get_scale_factor();
    let phys_x = (region.x as f64 * scale) as u32;
    let phys_y = (region.y as f64 * scale) as u32;
    let phys_w = (region.width as f64 * scale) as u32;
    let phys_h = (region.height as f64 * scale) as u32;

    wal.info("SNAP", &format!(
        "裁剪选区 | logical={},{},{},{} | physical={},{},{},{} | scale={}",
        region.x, region.y, region.width, region.height,
        phys_x, phys_y, phys_w, phys_h, scale
    ));

    // 先隐藏 overlay（异步，避免卡死）
    let app_for_close = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        hide_overlay(&app_for_close);
    });

    // 从预截的全屏图中裁剪
    let buffer = app.state::<ScreenBuffer>();
    let guard = buffer.image.lock().map_err(|e| e.to_string())?;
    let full_image = guard.as_ref().ok_or("没有预截的屏幕图像")?;

    let cropped = crop::crop_rgba(full_image, phys_x, phys_y, phys_w, phys_h)?;

    let config_manager = app.state::<ConfigManager>();
    let save_dir = config_manager.get_save_directory();
    let path = save::generate_snap_path(&save_dir);

    save::save_png(cropped.as_raw(), cropped.width(), cropped.height(), &path)?;
    clipboard::copy_image(cropped.as_raw(), cropped.width() as usize, cropped.height() as usize)?;

    wal.info("SNAP", &format!("截屏完成 | path={} | size={}x{}",
        path.to_string_lossy(), cropped.width(), cropped.height()));
    Ok(path.to_string_lossy().to_string())
}

/// F1 触发：先截全屏存内存，再显示 overlay
pub fn open_snap_overlay(app: &AppHandle) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("SNAP", "F1/菜单触发，预截全屏");

    let screen = crate::snap::capture::capture_primary_screen()?;
    let buffer = app.state::<ScreenBuffer>();
    let mut guard = buffer.image.lock().map_err(|e| e.to_string())?;
    *guard = Some(screen.image);

    wal.info("SNAP", "全屏预截完成，显示选区遮罩");

    // 先显示窗口，再通知前端切换模式（确保窗口尺寸就绪）
    show_overlay(app)?;
    // 稍等窗口展开
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _ = app.emit("overlay-mode", "snap-overlay");
    Ok(())
}
