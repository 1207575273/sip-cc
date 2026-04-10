use image::{Rgba, RgbaImage};
use xcap::Monitor;

/// xcap 0.9+：`x/y/width/height` 返回 `Result`，统一解包
pub(crate) fn monitor_x(m: &Monitor) -> Result<i32, String> {
    m.x().map_err(|e| format!("xcap: {e}"))
}
pub(crate) fn monitor_y(m: &Monitor) -> Result<i32, String> {
    m.y().map_err(|e| format!("xcap: {e}"))
}
pub(crate) fn monitor_width(m: &Monitor) -> Result<u32, String> {
    m.width().map_err(|e| format!("xcap: {e}"))
}
pub(crate) fn monitor_height(m: &Monitor) -> Result<u32, String> {
    m.height().map_err(|e| format!("xcap: {e}"))
}

pub struct ScreenCapture {
    pub image: RgbaImage,
}

/// 虚拟桌面 bounding box（物理像素坐标）
#[derive(Debug, Clone)]
pub struct VirtualScreenBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 矩形区域（用于相交计算）
#[derive(Debug, Clone, Copy)]
pub struct PhysRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl PhysRect {
    /// 计算两个矩形的交集，无交集返回 None
    pub fn intersect(&self, other: &PhysRect) -> Option<PhysRect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.w).min(other.x + other.w);
        let y2 = (self.y + self.h).min(other.y + other.h);
        if x2 > x1 && y2 > y1 {
            Some(PhysRect {
                x: x1,
                y: y1,
                w: x2 - x1,
                h: y2 - y1,
            })
        } else {
            None
        }
    }
}

/// 获取所有显示器
pub fn get_all_monitors() -> Result<Vec<Monitor>, String> {
    Monitor::all().map_err(|e| format!("获取显示器列表失败: {e}"))
}

/// 获取主显示器
#[allow(dead_code)] // 供 capture_primary_screen / find_monitor_at 使用，对外 API 保留
fn get_primary_monitor() -> Result<Monitor, String> {
    get_all_monitors()?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or_else(|| "未找到主显示器".to_string())
}

/// 计算所有显示器的虚拟桌面 bounding box
pub fn get_virtual_screen_bounds() -> Result<VirtualScreenBounds, String> {
    let monitors = get_all_monitors()?;
    if monitors.is_empty() {
        return Err("没有检测到任何显示器".to_string());
    }
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for m in &monitors {
        let mx = monitor_x(m)?;
        let my = monitor_y(m)?;
        let mw = monitor_width(m)?;
        let mh = monitor_height(m)?;
        min_x = min_x.min(mx);
        min_y = min_y.min(my);
        max_x = max_x.max(mx + mw as i32);
        max_y = max_y.max(my + mh as i32);
    }
    Ok(VirtualScreenBounds {
        x: min_x,
        y: min_y,
        width: (max_x - min_x) as u32,
        height: (max_y - min_y) as u32,
    })
}

/// 根据虚拟桌面物理坐标查找所在显示器
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn find_monitor_at(x: i32, y: i32) -> Result<Monitor, String> {
    let monitors = get_all_monitors()?;
    for m in &monitors {
        let mx = monitor_x(m)?;
        let my = monitor_y(m)?;
        let mw = monitor_width(m)?;
        let mh = monitor_height(m)?;
        if x >= mx && x < mx + mw as i32 && y >= my && y < my + mh as i32 {
            return Ok(m.clone());
        }
    }
    get_primary_monitor()
}

/// 判断选区是否完全在单个显示器内（快速路径）
fn find_single_containing_monitor(
    monitors: &[Monitor],
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Result<Option<usize>, String> {
    for (i, m) in monitors.iter().enumerate() {
        let mx = monitor_x(m)?;
        let my = monitor_y(m)?;
        let mw = monitor_width(m)?;
        let mh = monitor_height(m)?;
        if x >= mx
            && y >= my
            && x + w as i32 <= mx + mw as i32
            && y + h as i32 <= my + mh as i32
        {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// 截取主显示器全屏（向后兼容，单屏场景可调用）
#[allow(dead_code)]
pub fn capture_primary_screen() -> Result<ScreenCapture, String> {
    let monitor = get_primary_monitor()?;
    capture_screen(&monitor)
}

/// 截取指定显示器全屏
pub fn capture_screen(monitor: &Monitor) -> Result<ScreenCapture, String> {
    let image = monitor
        .capture_image()
        .map_err(|e| format!("屏幕截取失败: {e}"))?;
    Ok(ScreenCapture { image })
}

/// 截取主显示器指定区域（向后兼容）
#[allow(dead_code)]
pub fn capture_region(x: u32, y: u32, width: u32, height: u32) -> Result<RgbaImage, String> {
    let monitor = get_primary_monitor()?;
    capture_region_from(&monitor, x, y, width, height)
}

/// 从指定显示器截取局部区域
pub fn capture_region_from(
    monitor: &Monitor,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, String> {
    monitor
        .capture_region(x, y, width, height)
        .map_err(|e| format!("区域截取失败: {e}"))
}

/// 截取所有显示器并拼接为虚拟桌面大图
pub fn capture_virtual_screen() -> Result<(RgbaImage, VirtualScreenBounds), String> {
    let monitors = get_all_monitors()?;
    let bounds = get_virtual_screen_bounds()?;

    let mut canvas = RgbaImage::from_pixel(
        bounds.width,
        bounds.height,
        Rgba([0, 0, 0, 255]),
    );

    for monitor in &monitors {
        let image = monitor
            .capture_image()
            .map_err(|e| format!("截取显示器失败: {e}"))?;
        let mx = monitor_x(monitor)?;
        let my = monitor_y(monitor)?;
        let offset_x = (mx - bounds.x) as i64;
        let offset_y = (my - bounds.y) as i64;
        image::imageops::overlay(&mut canvas, &image, offset_x, offset_y);
    }

    Ok((canvas, bounds))
}

/// 从虚拟桌面坐标截取区域（可跨屏）。x, y 为虚拟桌面物理坐标。
pub fn capture_virtual_region(x: i32, y: i32, width: u32, height: u32) -> Result<RgbaImage, String> {
    let monitors = get_all_monitors()?;

    if let Some(idx) = find_single_containing_monitor(&monitors, x, y, width, height)? {
        let m = &monitors[idx];
        let mx = monitor_x(m)?;
        let my = monitor_y(m)?;
        let local_x = (x - mx) as u32;
        let local_y = (y - my) as u32;
        return capture_region_from(m, local_x, local_y, width, height);
    }

    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 255]));
    let sel = PhysRect {
        x,
        y,
        w: width as i32,
        h: height as i32,
    };

    for monitor in &monitors {
        let mx = monitor_x(monitor)?;
        let my = monitor_y(monitor)?;
        let mw = monitor_width(monitor)?;
        let mh = monitor_height(monitor)?;
        let mon = PhysRect {
            x: mx,
            y: my,
            w: mw as i32,
            h: mh as i32,
        };
        if let Some(overlap) = sel.intersect(&mon) {
            let local_x = (overlap.x - mx) as u32;
            let local_y = (overlap.y - my) as u32;
            let region = capture_region_from(
                monitor,
                local_x,
                local_y,
                overlap.w as u32,
                overlap.h as u32,
            )?;
            let canvas_x = (overlap.x - x) as i64;
            let canvas_y = (overlap.y - y) as i64;
            image::imageops::overlay(&mut canvas, &region, canvas_x, canvas_y);
        }
    }

    Ok(canvas)
}

/// 录制/GIF/视频每帧截取：Windows/Linux 用虚拟桌面跨屏；macOS 选区在同一显示器内，用局部截取。
pub fn capture_recording_frame(x: i32, y: i32, width: u32, height: u32) -> Result<RgbaImage, String> {
    #[cfg(target_os = "macos")]
    {
        let m = find_monitor_at(x, y)?;
        let mx = monitor_x(&m)?;
        let my = monitor_y(&m)?;
        let lx = (x - mx) as u32;
        let ly = (y - my) as u32;
        capture_region_from(&m, lx, ly, width, height)
    }
    #[cfg(not(target_os = "macos"))]
    {
        capture_virtual_region(x, y, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_intersect_overlapping_rects() {
        let a = PhysRect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let b = PhysRect {
            x: 50,
            y: 50,
            w: 100,
            h: 100,
        };
        let result = a.intersect(&b).unwrap();
        assert_eq!(result.x, 50);
        assert_eq!(result.y, 50);
        assert_eq!(result.w, 50);
        assert_eq!(result.h, 50);
    }

    #[test]
    fn should_return_none_for_non_overlapping() {
        let a = PhysRect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let b = PhysRect {
            x: 200,
            y: 200,
            w: 100,
            h: 100,
        };
        assert!(a.intersect(&b).is_none());
    }

    #[test]
    fn should_handle_contained_rect() {
        let outer = PhysRect {
            x: 0,
            y: 0,
            w: 200,
            h: 200,
        };
        let inner = PhysRect {
            x: 50,
            y: 50,
            w: 50,
            h: 50,
        };
        let result = outer.intersect(&inner).unwrap();
        assert_eq!(result.x, 50);
        assert_eq!(result.y, 50);
        assert_eq!(result.w, 50);
        assert_eq!(result.h, 50);
    }

    #[test]
    fn should_handle_edge_touching() {
        let a = PhysRect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let b = PhysRect {
            x: 100,
            y: 0,
            w: 100,
            h: 100,
        };
        assert!(a.intersect(&b).is_none());
    }

    #[test]
    fn should_handle_negative_coordinates() {
        let a = PhysRect {
            x: -1920,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let b = PhysRect {
            x: -100,
            y: 0,
            w: 200,
            h: 500,
        };
        let result = a.intersect(&b).unwrap();
        assert_eq!(result.x, -100);
        assert_eq!(result.y, 0);
        assert_eq!(result.w, 100);
        assert_eq!(result.h, 500);
    }
}
