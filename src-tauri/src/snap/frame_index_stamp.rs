//! 长截图调试：在帧左上角绘制序号（与 `EXCLUDE_*` 区域一致，便于与差异检测对齐）

use image::{Rgba, RgbaImage};

/// 与 [`stamp_frame_index_top_left`] 占用区域一致，供帧间差异计算时跳过（避免序号干扰）
pub const EXCLUDE_W: u32 = 128;
pub const EXCLUDE_H: u32 = 44;

const GLYPH: [[u8; 7]; 10] = [
    [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
    [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
    [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
    [0x11, 0x11, 0x11, 0x1F, 0x01, 0x01, 0x01],
    [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
    [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
    [0x1F, 0x01, 0x02, 0x04, 0x04, 0x04, 0x04],
    [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
    [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x11, 0x0E],
];

/// 在左上角绘制 1-based 序号（半透明底 + 白字），便于上下滑拼接时对照帧顺序
pub fn stamp_frame_index_top_left(img: &mut RgbaImage, one_based: u32) {
    let n = one_based.min(999).max(1);
    let s = format!("{}", n);
    let scale = 2u32;
    let gap = 2u32;
    let pad = 6u32;
    let cw = 5 * scale;
    let ch = 7 * scale;
    let nd = s.len() as u32;
    let bg_w = (pad * 2 + nd * cw + nd.saturating_sub(1) * gap).min(img.width());
    let bg_h = (pad * 2 + ch).min(img.height());

    for y in 0..bg_h {
        for x in 0..bg_w {
            blend_darken(img, x, y, 200);
        }
    }

    for (di, ch) in s.chars().enumerate() {
        let d = ch.to_digit(10).unwrap_or(0) as usize;
        if d >= 10 {
            continue;
        }
        let ox = pad + di as u32 * (cw + gap);
        if ox + cw > img.width() {
            break;
        }
        let oy = pad;
        for sy in 0..7u32 {
            let row = GLYPH[d][sy as usize];
            for sx in 0..5u32 {
                if (row >> (4 - sx)) & 1 == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = ox + sx * scale + dx;
                        let py = oy + sy * scale + dy;
                        if px < img.width() && py < img.height() {
                            img.put_pixel(px, py, Rgba([255, 255, 255, 255]));
                        }
                    }
                }
            }
        }
    }
}

fn blend_darken(img: &mut RgbaImage, x: u32, y: u32, a: u8) {
    let p = *img.get_pixel(x, y);
    let ia = a as u32;
    let oa = 255u32 - ia;
    let nr = ((0u32 * ia + p[0] as u32 * oa) / 255) as u8;
    let ng = ((0u32 * ia + p[1] as u32 * oa) / 255) as u8;
    let nb = ((0u32 * ia + p[2] as u32 * oa) / 255) as u8;
    img.put_pixel(x, y, Rgba([nr, ng, nb, 255]));
}
