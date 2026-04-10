//! 纵向长截图：算法抽象（[`algo`] / [`ssd`]）与采集无关工具。
//!
//! ## 模块划分
//! - [`algo`]：**输入 / 输出 / [`VerticalStitcher`] trait** — 与平台解耦。
//! - [`ssd`]：默认 SSD 条带实现 [`SsdOverlapStitcher`]。
//! - 本文件：滚到底检测等**非拼接**工具、默认入口 [`stitch_default`]、兼容旧 API。

pub mod algo;
pub mod ssd;

pub use algo::{StitchError, StitchInput, VerticalStitcher};
pub use ssd::SsdOverlapStitcher;

use image::RgbaImage;

/// 默认：重叠搜索下限（行数）
pub const DEFAULT_MIN_OVERLAP: u32 = 32;
/// 相邻两帧几乎相同则视为滚到底（平均通道差）；与 `LONG_SNAP_MIN_MEAN_DIFF_APPEND` 可对照调参。
#[allow(dead_code)]
pub const DEFAULT_BOTTOM_MEAN_DIFF: f64 = 1.8;
/// 默认：重叠区归一化 SSD 超过此值则认为对不齐
pub const DEFAULT_MAX_OVERLAP_SSD: f64 = 800.0;

/// **默认拼接算法**（更换实现时只改此函数体或改 [`stitch_with`] 的分发）。
#[inline]
pub fn stitch_default(input: StitchInput<'_>) -> Result<RgbaImage, StitchError> {
    stitch_with(&SsdOverlapStitcher, input)
}

/// 显式注入 [`VerticalStitcher`]（测试或 A/B 不同实现时使用）。
#[inline]
pub fn stitch_with(
    stitcher: &impl VerticalStitcher,
    input: StitchInput<'_>,
) -> Result<RgbaImage, StitchError> {
    stitcher.stitch(input)
}

/// 两图同尺寸时，全图平均绝对通道差（调试或全图对比用；滚轮采样请用 [`mean_abs_diff_excluding_rect`] 避开序号区）。
#[allow(dead_code)]
pub fn mean_abs_diff_full(a: &RgbaImage, b: &RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut sum: u64 = 0;
    let n = (a.width() * a.height()) as u64;
    if n == 0 {
        return 0.0;
    }
    for y in 0..a.height() {
        for x in 0..a.width() {
            let pa = a.get_pixel(x, y);
            let pb = b.get_pixel(x, y);
            sum += (pa[0] as i32 - pb[0] as i32).unsigned_abs() as u64;
            sum += (pa[1] as i32 - pb[1] as i32).unsigned_abs() as u64;
            sum += (pa[2] as i32 - pb[2] as i32).unsigned_abs() as u64;
        }
    }
    (sum as f64) / (n as f64 * 3.0)
}

/// 排除左上角矩形后的平均通道差（用于帧间比较：跳过序号水印区域）
pub fn mean_abs_diff_excluding_rect(
    a: &RgbaImage,
    b: &RgbaImage,
    exclude_x: u32,
    exclude_y: u32,
    exclude_w: u32,
    exclude_h: u32,
) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions());
    let w = a.width();
    let h = a.height();
    let ex2 = exclude_x.saturating_add(exclude_w).min(w);
    let ey2 = exclude_y.saturating_add(exclude_h).min(h);
    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    for y in 0..h {
        for x in 0..w {
            if x >= exclude_x && x < ex2 && y >= exclude_y && y < ey2 {
                continue;
            }
            let pa = a.get_pixel(x, y);
            let pb = b.get_pixel(x, y);
            sum += (pa[0] as i32 - pb[0] as i32).unsigned_abs() as u64;
            sum += (pa[1] as i32 - pb[1] as i32).unsigned_abs() as u64;
            sum += (pa[2] as i32 - pb[2] as i32).unsigned_abs() as u64;
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    (sum as f64) / (count as f64 * 3.0)
}

/// 兼容旧调用：使用默认 SSD 算法，错误为 `String`。
#[allow(dead_code)] // 对外保留；单元测试会引用，但 `lib` 非 test 构建中未调用
pub fn stitch_vertical_frames(
    frames: &[RgbaImage],
    min_overlap: u32,
    max_ssd: f64,
) -> Result<RgbaImage, String> {
    stitch_default(StitchInput {
        frames,
        min_overlap,
        max_overlap_ssd: max_ssd,
    })
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod compat_tests {
    use image::{Rgba, RgbaImage};

    use super::stitch_vertical_frames;

    #[test]
    fn stitch_vertical_frames_single_frame_ok() {
        let img = RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]));
        let out = stitch_vertical_frames(&[img], 1, 10_000.0).unwrap();
        assert_eq!(out.dimensions(), (4, 4));
    }
}
