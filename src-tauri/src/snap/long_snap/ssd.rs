//! 默认实现：基于重叠条带灰度 SSD 的纵向拼接（与历史行为一致，可整体替换为其它 [`super::algo::VerticalStitcher`]）

use image::{imageops, RgbaImage};

use super::algo::{StitchError, StitchInput, VerticalStitcher};

fn gray(p: &image::Rgba<u8>) -> i32 {
    (p[0] as i32 + p[1] as i32 + p[2] as i32) / 3
}

fn overlap_ssd(top: &RgbaImage, bottom: &RgbaImage, l: u32) -> f64 {
    let h = top.height();
    let w = top.width();
    debug_assert_eq!(bottom.height(), h);
    if l == 0 || l > h {
        return f64::MAX;
    }
    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    for y in 0..l {
        let y_top = h - l + y;
        for x in 0..w {
            let pa = top.get_pixel(x, y_top);
            let pb = bottom.get_pixel(x, y);
            let d = gray(&pa) - gray(&pb);
            sum += (d * d) as u64;
            count += 1;
        }
    }
    if count == 0 {
        return f64::MAX;
    }
    sum as f64 / count as f64
}

/// SSD 并列判断阈值：灰度差平方均值在此范围内视为"相同分数"，取更大重叠。
/// SSD 值域 0~65025，1e-6 约等于严格相等，仅处理浮点舍入误差。
const SSD_TIE_EPSILON: f64 = 1e-6;

/// 最大搜索重叠行数占帧高度的比例（3/4），实际滚动重叠不会接近帧高度
fn max_search_overlap(h: u32) -> u32 {
    h * 3 / 4
}

fn find_best_overlap(
    top: &RgbaImage,
    bottom: &RgbaImage,
    min_l: u32,
    max_ssd: f64,
) -> Result<u32, StitchError> {
    let h = top.height();
    if bottom.height() != h || top.width() != bottom.width() {
        return Err(StitchError::SizeMismatch);
    }
    if h <= min_l + 1 {
        return Err(StitchError::RegionTooSmall {
            min_height: min_l + 2,
        });
    }
    // 兜底：即使 max_search_overlap 算出来比 min_l 还小，也至少搜索 1 个值
    let max_l = max_search_overlap(h).max(min_l + 1);
    // 单轮遍历：同时追踪最小 SSD 和对应的最大重叠行数
    let mut min_score = f64::MAX;
    let mut best_l = min_l;
    for l in min_l..max_l {
        let s = overlap_ssd(top, bottom, l);
        if s < min_score {
            min_score = s;
            best_l = l;
        } else if (s - min_score).abs() < SSD_TIE_EPSILON && l > best_l {
            // 并列最小 SSD 时取最大重叠（更多内容对齐）
            best_l = l;
        }
    }
    if min_score > max_ssd {
        return Err(StitchError::AlignmentFailed {
            detail: format!(
                "无法对齐相邻帧（重叠误差过大: {:.1}，请换静态区域或缩短选区）",
                min_score
            ),
        });
    }
    Ok(best_l)
}

fn vertical_concat(top: &RgbaImage, strip: &RgbaImage) -> Result<RgbaImage, StitchError> {
    if top.width() != strip.width() {
        return Err(StitchError::SizeMismatch);
    }
    let w = top.width();
    let ha = top.height();
    let hb = strip.height();
    let mut out = RgbaImage::new(w, ha + hb);
    imageops::overlay(&mut out, top, 0, 0);
    imageops::overlay(&mut out, strip, 0, ha as i64);
    Ok(out)
}

/// 基于条带 SSD 与「最小 SSD 并列取最大重叠」的纵向拼接器（**当前默认**）。
#[derive(Debug, Clone, Copy, Default)]
pub struct SsdOverlapStitcher;

impl VerticalStitcher for SsdOverlapStitcher {
    fn stitch(&self, input: StitchInput<'_>) -> Result<RgbaImage, StitchError> {
        let frames = input.frames;
        if frames.is_empty() {
            return Err(StitchError::EmptyFrames);
        }
        if frames.len() == 1 {
            return Ok(frames[0].clone());
        }
        let w = frames[0].width();
        let h = frames[0].height();
        for f in frames {
            if f.width() != w || f.height() != h {
                return Err(StitchError::SizeMismatch);
            }
        }

        let mut acc = frames[0].clone();
        for i in 1..frames.len() {
            let overlap = find_best_overlap(
                &frames[i - 1],
                &frames[i],
                input.min_overlap,
                input.max_overlap_ssd,
            )?;
            let new_h = h - overlap;
            if new_h == 0 {
                continue;
            }
            let strip = imageops::crop_imm(&frames[i], 0, overlap, w, new_h).to_image();
            acc = vertical_concat(&acc, &strip)?;
        }
        Ok(acc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(c))
    }

    #[test]
    fn ssd_stitch_two_frames() {
        let w = 40u32;
        let h = 20u32;
        let top = solid(w, h, [100, 100, 100, 255]);
        let mut bottom = solid(w, h, [200, 200, 200, 255]);
        for y in 0..10 {
            for x in 0..w {
                *bottom.get_pixel_mut(x, y) = *top.get_pixel(x, h - 10 + y);
            }
        }
        let out = SsdOverlapStitcher
            .stitch(StitchInput {
                frames: &[top.clone(), bottom],
                min_overlap: 4,
                max_overlap_ssd: 10_000.0,
            })
            .unwrap();
        assert_eq!(out.width(), w);
        assert_eq!(out.height(), h + 10);
    }

    #[test]
    fn solid_color_frames_should_not_overlap_fully() {
        // 纯色帧：所有重叠行 SSD 均为 0，修复前会选到 h-1 导致几乎无新内容
        let w = 40u32;
        let h = 20u32;
        let a = solid(w, h, [128, 128, 128, 255]);
        let b = solid(w, h, [128, 128, 128, 255]);
        let out = SsdOverlapStitcher
            .stitch(StitchInput {
                frames: &[a, b],
                min_overlap: 4,
                max_overlap_ssd: 10_000.0,
            })
            .unwrap();
        // max_l = max_search_overlap(20) = 15，遍历 4..15，纯色 SSD 全为 0
        // 并列取最大 → best_l = 14，输出 = 20 + (20 - 14) = 26
        assert_eq!(out.height(), 26);
    }

    #[test]
    fn find_best_overlap_single_pass_matches_expected() {
        let w = 10u32;
        let h = 20u32;
        let top = solid(w, h, [50, 50, 50, 255]);
        let mut bottom = solid(w, h, [200, 200, 200, 255]);
        // 制造 6 行重叠
        for y in 0..6 {
            for x in 0..w {
                *bottom.get_pixel_mut(x, y) = *top.get_pixel(x, h - 6 + y);
            }
        }
        let overlap = find_best_overlap(&top, &bottom, 4, 10_000.0).unwrap();
        assert_eq!(overlap, 6);
    }

    #[test]
    fn overlap_beyond_max_ratio_should_be_clamped() {
        // 真实重叠 16 行超过 max_search_overlap(20) = 15，搜索范围 4..15 找不到完美匹配
        // 验证 max_l 截断后仍能选出搜索范围内的最佳值
        let w = 10u32;
        let h = 20u32;
        let top = solid(w, h, [80, 80, 80, 255]);
        let mut bottom = solid(w, h, [220, 220, 220, 255]);
        // 制造 16 行重叠（超过 3/4 = 15 的上限）
        for y in 0..16 {
            for x in 0..w {
                *bottom.get_pixel_mut(x, y) = *top.get_pixel(x, h - 16 + y);
            }
        }
        let overlap = find_best_overlap(&top, &bottom, 4, 10_000.0).unwrap();
        // 搜索范围 4..15，l=4..14 中 4..15 行都在重叠区内（SSD=0），取最大 → 14
        assert_eq!(overlap, 14);
    }
}
