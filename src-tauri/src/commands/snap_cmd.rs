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

#[tauri::command]
pub fn snap_region(app: AppHandle, region: Region) -> Result<String, String> {
    let wal = app.state::<WalLogger>();
    wal.info(
        "SNAP",
        &format!(
            "开始截屏 | region={},{},{},{}",
            region.x, region.y, region.width, region.height
        ),
    );

    let screen = capture::capture_primary_screen().map_err(|e| {
        wal.error("SNAP", &format!("截屏失败 | reason={e}"));
        e
    })?;

    let cropped = crop::crop_rgba(
        &screen.image,
        region.x as u32,
        region.y as u32,
        region.width,
        region.height,
    )?;

    let config_manager = app.state::<ConfigManager>();
    let save_dir = config_manager.get_save_directory();
    let path = save::generate_snap_path(&save_dir);

    save::save_png(cropped.as_raw(), cropped.width(), cropped.height(), &path)?;
    clipboard::copy_image(
        cropped.as_raw(),
        cropped.width() as usize,
        cropped.height() as usize,
    )?;

    wal.info(
        "SNAP",
        &format!("截屏完成 | path={}", path.to_string_lossy()),
    );
    Ok(path.to_string_lossy().to_string())
}

pub fn open_snap_overlay(app: &AppHandle) -> Result<(), String> {
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
        app,
        "snap-selection",
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
