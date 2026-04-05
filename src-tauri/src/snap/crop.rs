use image::{GenericImageView, RgbaImage};

pub fn crop_rgba(
    source: &RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, String> {
    let src_w = source.width();
    let src_h = source.height();

    let safe_x = x.min(src_w.saturating_sub(1));
    let safe_y = y.min(src_h.saturating_sub(1));
    let safe_w = width.min(src_w - safe_x);
    let safe_h = height.min(src_h - safe_y);

    let cropped = source.view(safe_x, safe_y, safe_w, safe_h).to_image();
    Ok(cropped)
}
