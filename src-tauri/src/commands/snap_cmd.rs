use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::ConfigManager;
use crate::output::{clipboard, save};
use crate::snap::capture::{
    self, capture_screen, capture_virtual_screen, get_all_monitors, monitor_height, monitor_width,
    monitor_x, monitor_y, VirtualScreenBounds,
};
use crate::snap::crop;
use crate::wal::logger::WalLogger;

const WINDOW_HIDE_DELAY_MS: u64 = 50;
const OVERLAY_SHOW_DELAY_MS: u64 = 50;

/// 启动时写入：macOS 预创建的 `overlay-0..overlay-{n-1}` 数量，与 `hide_overlay`/退出清理一致，避免「只关固定 8 个」与真实屏数不一致。
pub struct MacosOverlayCount(pub Mutex<usize>);

/// 供 hide / 托盘退出：与启动时创建的 overlay 数量一致；若状态未写入则回退为当前 `available_monitors().len()`。
pub fn macos_overlay_window_count(app: &AppHandle) -> usize {
    if !cfg!(target_os = "macos") {
        return 0;
    }
    if let Some(st) = app.try_state::<MacosOverlayCount>() {
        if let Ok(g) = st.0.lock() {
            if *g > 0 {
                return *g;
            }
        }
    }
    app.available_monitors().map(|m| m.len()).unwrap_or(0)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub monitor_index: Option<u32>,
}

pub struct ScreenBuffer {
    /// Windows/Linux：虚拟桌面拼接图；macOS：不使用
    pub image: Mutex<Option<RgbaImage>>,
    /// Windows/Linux：虚拟桌面范围；`snap_region` 中与预截图尺寸交叉校验，异常时打 WARN
    pub bounds: Mutex<Option<VirtualScreenBounds>>,
    /// macOS：按 `available_monitors()` 顺序的各屏全屏截图
    pub macos_monitors: Mutex<Option<Vec<RgbaImage>>>,
}

pub fn get_scale_factor(app: &AppHandle) -> f64 {
    app.primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0)
}

/// 将 overlay 内逻辑坐标转为物理像素时的 scale：macOS 可按屏取 `scale_factor`；Windows/Linux 单窗口跨虚拟桌面时用主屏 scale（见 `show_overlay_virtual_desktop` 注释）。
pub fn scale_for_overlay(app: &AppHandle, monitor_index: Option<u32>) -> f64 {
    if cfg!(target_os = "macos") {
        if let Some(idx) = monitor_index {
            if let Ok(monitors) = app.available_monitors() {
                if let Some(m) = monitors.get(idx as usize) {
                    return m.scale_factor();
                }
            }
        }
    }
    get_scale_factor(app)
}

/// 录制区域原点对应的虚拟桌面 / 单屏物理原点
pub fn get_virtual_bounds_for_recording(
    app: &AppHandle,
    monitor_index: Option<u32>,
) -> Result<VirtualScreenBounds, String> {
    if cfg!(target_os = "macos") {
        let monitors: Vec<_> = app
            .available_monitors()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let idx = monitor_index.unwrap_or(0) as usize;
        let m = monitors
            .get(idx)
            .ok_or_else(|| "无效的显示器索引".to_string())?;
        let pos = m.position();
        let sz = m.size();
        Ok(VirtualScreenBounds {
            x: pos.x,
            y: pos.y,
            width: sz.width,
            height: sz.height,
        })
    } else {
        capture::get_virtual_screen_bounds()
    }
}

/// 多屏 macOS 且未带 `monitor_index`（非标准入口）时提示：避免静默用主屏 DPI 导致错位。
pub fn warn_if_macos_missing_monitor_index(
    app: &AppHandle,
    wal: &WalLogger,
    tag: &str,
    monitor_index: Option<u32>,
) {
    if !cfg!(target_os = "macos") || monitor_index.is_some() {
        return;
    }
    let n = app.available_monitors().map(|m| m.len()).unwrap_or(0);
    if n > 1 {
        wal.warn(
            tag,
            "多显示器且未指定 monitor_index，已按主显示器 DPI 回退；若选区错位请用快捷键/托盘正常打开选区",
        );
    }
}

#[derive(Debug, Serialize)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

#[tauri::command]
pub fn get_screen_info(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let primary = app.primary_monitor().ok().flatten();
    let primary_pos = primary.as_ref().map(|p| p.position());

    let result: Vec<MonitorInfo> = monitors
        .iter()
        .map(|m| {
            let is_primary = primary_pos
                .as_ref()
                .map(|pp| pp.x == m.position().x && pp.y == m.position().y)
                .unwrap_or(false);
            let phys = m.size();
            MonitorInfo {
                x: m.position().x,
                y: m.position().y,
                width: phys.width,
                height: phys.height,
                scale_factor: m.scale_factor(),
                is_primary,
            }
        })
        .collect();

    Ok(result)
}

/// 显示 overlay 窗口（公开供 gif_cmd / video_cmd 复用）
pub fn open_overlay_window(app: &AppHandle) -> Result<(), String> {
    show_overlay(app)
}

fn show_overlay(app: &AppHandle) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        show_overlay_per_monitor(app)
    } else {
        show_overlay_virtual_desktop(app)
    }
}

/// Windows/Linux：单 overlay 覆盖虚拟桌面。
///
/// **架构说明**：WebView 整窗只有一个逻辑坐标系，此处用主显示器 `scale_factor` 做物理↔逻辑换算。
/// 多屏 **混合 DPI** 时，系统可能对虚拟桌面有补偿，但 WebView 仍可能单 DPR，若出现选区与截图偏差需在「每屏独立 overlay」或「按点取 scale」方向演进（成本较高，见设计文档）。
fn show_overlay_virtual_desktop(app: &AppHandle) -> Result<(), String> {
    let bounds = capture::get_virtual_screen_bounds()?;
    let win = app
        .get_webview_window("overlay")
        .ok_or("overlay 窗口不存在")?;

    let scale = get_scale_factor(app);
    let logical_x = bounds.x as f64 / scale;
    let logical_y = bounds.y as f64 / scale;
    let logical_w = bounds.width as f64 / scale;
    let logical_h = bounds.height as f64 / scale;

    let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
        logical_x, logical_y,
    )));
    let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        logical_w, logical_h,
    )));
    let _ = win.set_always_on_top(true);
    win.show()
        .map_err(|e| format!("显示 overlay 失败: {e}"))?;
    let _ = win.set_focus();
    Ok(())
}

/// macOS：显示所有 overlay-N
fn show_overlay_per_monitor(app: &AppHandle) -> Result<(), String> {
    let monitors: Vec<_> = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    for (i, monitor) in monitors.iter().enumerate() {
        let label = format!("overlay-{}", i);
        if let Some(win) = app.get_webview_window(&label) {
            let phys = monitor.size();
            let scale = monitor.scale_factor();
            let pos = monitor.position();
            let logical_x = pos.x as f64 / scale;
            let logical_y = pos.y as f64 / scale;
            let logical_w = phys.width as f64 / scale;
            let logical_h = phys.height as f64 / scale;

            let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
                logical_x, logical_y,
            )));
            let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                logical_w, logical_h,
            )));
            let _ = win.set_always_on_top(true);
            let _ = win.show();
        }
    }

    if let Some(win) = app.get_webview_window("overlay-0") {
        let _ = win.set_focus();
    }

    Ok(())
}

/// 鼠标/滚轮是否穿透 overlay 落到下层窗口（CSS pointer-events 无法实现系统级穿透，需用此 API）。
pub fn set_overlay_cursor_passthrough_inner(app: &AppHandle, passthrough: bool) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        let n = macos_overlay_window_count(app);
        for i in 0..n {
            let label = format!("overlay-{}", i);
            if let Some(win) = app.get_webview_window(&label) {
                win.set_ignore_cursor_events(passthrough)
                    .map_err(|e| e.to_string())?;
            }
        }
    } else if let Some(win) = app.get_webview_window("overlay") {
        win.set_ignore_cursor_events(passthrough)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_overlay_cursor_passthrough(app: AppHandle, passthrough: bool) -> Result<(), String> {
    set_overlay_cursor_passthrough_inner(&app, passthrough)
}

/// 隐藏 overlay 窗口
pub fn hide_overlay(app: &AppHandle) {
    let _ = set_overlay_cursor_passthrough_inner(app, false);
    crate::commands::long_snap_cmd::reset_long_snap_on_overlay_hide(app);
    if cfg!(target_os = "macos") {
        let n = macos_overlay_window_count(app);
        for i in 0..n {
            let label = format!("overlay-{}", i);
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.set_always_on_top(false);
                let _ = win.hide();
            }
        }
    } else if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.set_always_on_top(false);
        let _ = win.hide();
    }
}

/// 关闭录制相关的浮动窗口
pub fn close_floating_windows(app: &AppHandle) {
    for label in &["record-bar", "record-region", "record-info", "long-snap-control"] {
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

/// macOS：按 Tauri 显示器顺序截取各屏，顺序与 overlay-N 一致
fn capture_macos_per_monitor_images(app: &AppHandle) -> Result<Vec<RgbaImage>, String> {
    let tauri_mons: Vec<_> = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let xcap_all = get_all_monitors()?;

    let mut images = Vec::with_capacity(tauri_mons.len());
    for tm in &tauri_mons {
        let pos = tm.position();
        let sz = tm.size();
        let xm = xcap_all
            .iter()
            .find(|m| {
                monitor_x(m).map(|x| x == pos.x).unwrap_or(false)
                    && monitor_y(m).map(|y| y == pos.y).unwrap_or(false)
                    && monitor_width(m).map(|w| w == sz.width).unwrap_or(false)
                    && monitor_height(m).map(|h| h == sz.height).unwrap_or(false)
            })
            .or_else(|| {
                xcap_all.iter().find(|m| {
                    monitor_x(m).map(|x| x == pos.x).unwrap_or(false)
                        && monitor_y(m).map(|y| y == pos.y).unwrap_or(false)
                })
            })
            .ok_or_else(|| {
                format!(
                    "xcap 未找到与 Tauri 匹配的显示器 ({}, {})",
                    pos.x, pos.y
                )
            })?;
        let sc = capture_screen(xm)?;
        images.push(sc.image);
    }
    Ok(images)
}

#[tauri::command]
pub fn snap_region(app: AppHandle, region: Region) -> Result<String, String> {
    let wal = app.state::<WalLogger>();
    warn_if_macos_missing_monitor_index(&app, &wal, "SNAP", region.monitor_index);

    let scale = scale_for_overlay(&app, region.monitor_index);
    let phys_x = (region.x as f64 * scale) as u32;
    let phys_y = (region.y as f64 * scale) as u32;
    let phys_w = (region.width as f64 * scale) as u32;
    let phys_h = (region.height as f64 * scale) as u32;

    wal.info(
        "SNAP",
        &format!(
            "裁剪选区 | logical={},{},{},{} | physical={},{},{},{} | scale={} | mon={:?}",
            region.x,
            region.y,
            region.width,
            region.height,
            phys_x,
            phys_y,
            phys_w,
            phys_h,
            scale,
            region.monitor_index
        ),
    );

    let app_for_close = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(WINDOW_HIDE_DELAY_MS));
        hide_overlay(&app_for_close);
    });

    let buffer = app.state::<ScreenBuffer>();

    let cropped = if cfg!(target_os = "macos") {
        let idx = region.monitor_index.unwrap_or(0) as usize;
        let guard = buffer.macos_monitors.lock().map_err(|e| e.to_string())?;
        let imgs = guard.as_ref().ok_or("没有预截的屏幕图像")?;
        let full_image = imgs.get(idx).ok_or("显示器索引超出范围")?;
        crop::crop_rgba(full_image, phys_x, phys_y, phys_w, phys_h)?
    } else {
        // 锁顺序：先 image 再 bounds，与其它路径一致
        let img_guard = buffer.image.lock().map_err(|e| e.to_string())?;
        let full_image = img_guard.as_ref().ok_or("没有预截的屏幕图像")?;
        let bounds_guard = buffer.bounds.lock().map_err(|e| e.to_string())?;
        if let Some(b) = bounds_guard.as_ref() {
            if full_image.width() != b.width || full_image.height() != b.height {
                wal.warn(
                    "SNAP",
                    &format!(
                        "预截图尺寸与虚拟桌面 bounds 不一致: img={}x{} bounds={}x{}",
                        full_image.width(),
                        full_image.height(),
                        b.width,
                        b.height
                    ),
                );
            }
        }
        drop(bounds_guard);
        crop::crop_rgba(full_image, phys_x, phys_y, phys_w, phys_h)?
    };

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
        &format!(
            "截屏完成 | path={} | size={}x{}",
            path.to_string_lossy(),
            cropped.width(),
            cropped.height()
        ),
    );
    Ok(path.to_string_lossy().to_string())
}

/// F1 触发：先截全屏存内存，再显示 overlay
pub fn open_snap_overlay(app: &AppHandle) -> Result<(), String> {
    let wal = app.state::<WalLogger>();
    wal.info("SNAP", "F1/菜单触发，预截屏幕");

    let buffer = app.state::<ScreenBuffer>();

    if cfg!(target_os = "macos") {
        let images = capture_macos_per_monitor_images(app)?;
        wal.info(
            "SNAP",
            &format!("macOS 各显示器预截完成 | count={}", images.len()),
        );
        *buffer.macos_monitors.lock().map_err(|e| e.to_string())? = Some(images);
        *buffer.image.lock().map_err(|e| e.to_string())? = None;
        *buffer.bounds.lock().map_err(|e| e.to_string())? = None;
    } else {
        let (image, bounds) = capture_virtual_screen()?;
        wal.info(
            "SNAP",
            &format!(
                "虚拟桌面预截完成 | bounds={},{},{},{}",
                bounds.x, bounds.y, bounds.width, bounds.height
            ),
        );
        *buffer.image.lock().map_err(|e| e.to_string())? = Some(image);
        *buffer.bounds.lock().map_err(|e| e.to_string())? = Some(bounds);
        *buffer.macos_monitors.lock().map_err(|e| e.to_string())? = None;
    }

    show_overlay(app)?;
    std::thread::sleep(std::time::Duration::from_millis(OVERLAY_SHOW_DELAY_MS));
    let _ = app.emit("overlay-mode", "snap-overlay");
    Ok(())
}
