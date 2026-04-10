//! Windows：手动滚动长截图 —— 采集固定矩形；取景框在 overlay 采集层，用户在框内滚动物理层内容后逐帧捕获。

use image::RgbaImage;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::snap_cmd::{scale_for_overlay, warn_if_macos_missing_monitor_index, Region};
use crate::config::ConfigManager;
use crate::output::{clipboard, save};
use crate::snap::capture::capture_virtual_region;
use crate::snap::long_snap::{
    stitch_default, StitchInput, DEFAULT_MAX_OVERLAP_SSD, DEFAULT_MIN_OVERLAP,
};
use crate::wal::logger::WalLogger;

const MAX_FRAMES: usize = 80;
const MAX_OUTPUT_HEIGHT_PX: u32 = 32_000;

const LONG_SNAP_CTRL_W: f64 = 280.0;
const LONG_SNAP_CTRL_H: f64 = 44.0;
const BAR_OFFSET_Y: f64 = 6.0;

/// 独立小窗：不穿透，供「完成/取消」点击（主 overlay 整窗穿透后无法点按钮）
pub fn close_long_snap_control_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("long-snap-control") {
        let _ = w.close();
    }
}

pub fn open_long_snap_control_window(app: &AppHandle, region: &Region) -> Result<(), String> {
    close_long_snap_control_window(app);

    let log_x = region.x as f64;
    let log_y = region.y as f64;
    let log_w = region.width as f64;
    let log_h = region.height as f64;

    let bar_w = LONG_SNAP_CTRL_W.min(log_w.max(220.0));
    let bar_x = log_x + log_w - bar_w;
    let bar_y = log_y + log_h + BAR_OFFSET_Y;

    WebviewWindowBuilder::new(
        app,
        "long-snap-control",
        WebviewUrl::App("index.html?view=long-snap-control".into()),
    )
    .title("sip-cc long snap")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .inner_size(bar_w, LONG_SNAP_CTRL_H)
    .position(bar_x.max(0.0), bar_y)
    .build()
    .map_err(|e| format!("打开长截图控制条失败: {e}"))?;
    Ok(())
}

/// 选区物理像素（虚拟桌面坐标）
pub fn phys_rect_for_region(app: &AppHandle, region: &Region) -> (i32, i32, u32, u32) {
    let scale = scale_for_overlay(app, region.monitor_index);
    let phys_x = (region.x as f64 * scale) as i32;
    let phys_y = (region.y as f64 * scale) as i32;
    let phys_w = (region.width as f64 * scale) as u32;
    let phys_h = (region.height as f64 * scale) as u32;
    (phys_x, phys_y, phys_w, phys_h)
}

/// 截取当前选区一帧（手动长截图用）
pub fn capture_long_snap_frame(app: &AppHandle, region: &Region) -> Result<RgbaImage, String> {
    let wal = app.state::<WalLogger>();
    warn_if_macos_missing_monitor_index(app, &wal, "LONG_SNAP", region.monitor_index);
    let (px, py, pw, ph) = phys_rect_for_region(app, region);
    wal.info(
        "LONG_SNAP",
        &format!("捕获一帧 | phys={},{},{}x{}", px, py, pw, ph),
    );
    capture_virtual_region(px, py, pw, ph)
}

/// 拼接、保存、剪贴板
pub fn save_stitched_long_snap(app: &AppHandle, frames: Vec<RgbaImage>) -> Result<String, String> {
    let wal = app.state::<WalLogger>();
    if frames.is_empty() {
        return Err("没有可拼接的帧".to_string());
    }

    let stitched = stitch_default(StitchInput {
        frames: &frames,
        min_overlap: DEFAULT_MIN_OVERLAP,
        max_overlap_ssd: DEFAULT_MAX_OVERLAP_SSD,
    })
    .map_err(|e| e.to_string())?;

    if stitched.height() > MAX_OUTPUT_HEIGHT_PX {
        return Err(format!("输出高度超过上限 {}px", MAX_OUTPUT_HEIGHT_PX));
    }

    let config_manager = app.state::<ConfigManager>();
    let save_dir = config_manager.get_save_directory();
    let path = save::generate_snap_path(&save_dir);

    save::save_png(
        stitched.as_raw(),
        stitched.width(),
        stitched.height(),
        &path,
    )?;
    clipboard::copy_image(
        stitched.as_raw(),
        stitched.width() as usize,
        stitched.height() as usize,
    )?;

    wal.info(
        "LONG_SNAP",
        &format!(
            "手动长截图完成 | path={} | {}x{} | frames={}",
            path.to_string_lossy(),
            stitched.width(),
            stitched.height(),
            frames.len()
        ),
    );

    Ok(path.to_string_lossy().to_string())
}

pub const MAX_LONG_SNAP_FRAMES: usize = MAX_FRAMES;
