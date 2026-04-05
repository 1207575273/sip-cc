use image::RgbaImage;
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
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

/// 预截的全屏图像，F1 按下时先截好存这里
pub struct ScreenBuffer {
    pub image: Mutex<Option<RgbaImage>>,
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
        .and_then(|monitors| monitors.into_iter().find(|m| m.is_primary().unwrap_or(false)))
        .and_then(|m| m.scale_factor().ok())
        .unwrap_or(1.0) as f64
}

/// 从预截的全屏图中裁剪选区并保存（遮罩已经关了不需要再截屏）
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

    // 异步关闭 overlay（不阻塞命令返回，避免摧毁调用者环境）
    let app_for_close = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        close_all_overlays(&app_for_close);
    });

    // 从预截的全屏图中裁剪（不需要等窗口关闭）
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

#[tauri::command]
pub fn close_overlay(app: AppHandle) -> Result<(), String> {
    close_all_overlays(&app);
    Ok(())
}

/// F1 触发：先截全屏存内存，再打开遮罩让用户选区
pub fn open_snap_overlay(app: &AppHandle) -> Result<(), String> {
    close_all_overlays(app);

    let wal = app.state::<WalLogger>();
    wal.info("SNAP", "F1/菜单触发，预截全屏");

    // 先截全屏存到 ScreenBuffer
    let screen = crate::snap::capture::capture_primary_screen()?;
    let buffer = app.state::<ScreenBuffer>();
    let mut guard = buffer.image.lock().map_err(|e| e.to_string())?;
    *guard = Some(screen.image);

    wal.info("SNAP", "全屏预截完成，打开选区遮罩");

    // 再打开遮罩（此时屏幕截图已经存好了，遮罩不影响）
    WebviewWindowBuilder::new(
        app, "snap-selection",
        WebviewUrl::App("index.html?view=snap-overlay".into()),
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
