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
    let mut min_score = f64::MAX;
    for l in min_l..h {
        let s = overlap_ssd(top, bottom, l);
        if s < min_score {
            min_score = s;
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
    let mut best_l = min_l;
    for l in min_l..h {
        let s = overlap_ssd(top, bottom, l);
        if (s - min_score).abs() < 1e-6 && l > best_l {
            best_l = l;
        }
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
}
