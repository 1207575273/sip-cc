use image::RgbaImage;
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::ConfigManager;
use crate::output::{clipboard, save};
use crate::snap::crop;
use crate::wal::logger::WalLogger;

const WINDOW_HIDE_DELAY_MS: u64 = 50;
const OVERLAY_SHOW_DELAY_MS: u64 = 50;

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

pub fn get_scale_factor(app: &AppHandle) -> f64 {
    app.primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0)
}

/// 获取主显示器逻辑尺寸（用于 macOS 模拟全屏）
/// 使用 Tauri 的 Monitor API，避免 xcap 在 macOS 上物理/逻辑像素歧义
fn get_screen_logical_size(app: &AppHandle) -> (f64, f64) {
    app.primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let phys = m.size();
            let scale = m.scale_factor();
            (phys.width as f64 / scale, phys.height as f64 / scale)
        })
        .unwrap_or((1920.0, 1080.0))
}

/// 显示 overlay 窗口（公开供 gif_cmd 复用）
/// macOS：不用 fullscreen（会创建独立 Space 导致黑屏），用窗口尺寸覆盖屏幕
/// Windows/Linux：用 fullscreen
pub fn open_overlay_window(app: &AppHandle) -> Result<(), String> {
    show_overlay(app)
}

fn show_overlay(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("overlay") {
        if cfg!(target_os = "macos") {
            let (w, h) = get_screen_logical_size(app);
            let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(0.0, 0.0)));
            let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)));
        } else {
            let _ = win.set_fullscreen(true);
        }
        let _ = win.set_always_on_top(true);
        win.show().map_err(|e| format!("显示 overlay 失败: {e}"))?;
        let _ = win.set_focus();
        Ok(())
    } else {
        Err("overlay 窗口不存在".to_string())
    }
}

/// 隐藏 overlay 窗口
pub fn hide_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("overlay") {
        if !cfg!(target_os = "macos") {
            let _ = win.set_fullscreen(false);
        }
        let _ = win.set_always_on_top(false);
        let _ = win.hide();
    }
}

/// 关闭录制相关的浮动窗口
pub fn close_floating_windows(app: &AppHandle) {
    for label in &["record-bar", "record-region", "record-info"] {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.close();
        }
    }
}

#[tauri::command]
pub fn set_overlay_mode(app: AppHandle, mode: String) -> Result<(), String> {
    app.emit("overlay-mode", mode).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_overlay(app: AppHandle) -> Result<(), String> {
    hide_overlay(&app);
    Ok(())
}

#[tauri::command]
pub fn snap_region(app: AppHandle, region: Region) -> Result<String, String> {
    let wal = app.state::<WalLogger>();

    let scale = get_scale_factor(&app);
    let phys_x = (region.x as f64 * scale) as u32;
    let phys_y = (region.y as f64 * scale) as u32;
    let phys_w = (region.width as f64 * scale) as u32;
    let phys_h = (region.height as f64 * scale) as u32;

    wal.info("SNAP", &format!(
        "裁剪选区 | logical={},{},{},{} | physical={},{},{},{} | scale={}",
        region.x, region.y, region.width, region.height,
        phys_x, phys_y, phys_w, phys_h, scale
    ));

    let app_for_close = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(WINDOW_HIDE_DELAY_MS));
        hide_overlay(&app_for_close);
    });

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

    show_overlay(app)?;
    std::thread::sleep(std::time::Duration::from_millis(OVERLAY_SHOW_DELAY_MS));
    let _ = app.emit("overlay-mode", "snap-overlay");
    Ok(())
}
