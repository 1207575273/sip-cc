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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_image(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255]))
    }

    #[test]
    fn should_crop_normal_region() {
        let img = make_image(100, 100);
        let cropped = crop_rgba(&img, 10, 10, 50, 50).unwrap();
        assert_eq!(cropped.width(), 50);
        assert_eq!(cropped.height(), 50);
    }

    #[test]
    fn should_crop_full_image() {
        let img = make_image(200, 150);
        let cropped = crop_rgba(&img, 0, 0, 200, 150).unwrap();
        assert_eq!(cropped.width(), 200);
        assert_eq!(cropped.height(), 150);
    }

    #[test]
    fn should_clamp_when_exceeding_bounds() {
        let img = make_image(100, 100);
        // 请求超出边界的区域，应该被 clamp
        let cropped = crop_rgba(&img, 80, 80, 50, 50).unwrap();
        assert_eq!(cropped.width(), 20);  // 100 - 80 = 20
        assert_eq!(cropped.height(), 20);
    }

    #[test]
    fn should_clamp_when_xy_exceeds_image() {
        let img = make_image(100, 100);
        // x=200 超出图像，clamp 到 99
        let cropped = crop_rgba(&img, 200, 200, 50, 50).unwrap();
        assert_eq!(cropped.width(), 1);  // min(50, 100-99) = 1
        assert_eq!(cropped.height(), 1);
    }

    #[test]
    fn should_crop_zero_origin() {
        let img = make_image(50, 50);
        let cropped = crop_rgba(&img, 0, 0, 25, 25).unwrap();
        assert_eq!(cropped.width(), 25);
        assert_eq!(cropped.height(), 25);
    }
}
