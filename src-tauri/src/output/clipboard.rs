use arboard::{Clipboard, ImageData};
use std::borrow::Cow;

pub fn copy_image(pixels: &[u8], width: usize, height: usize) -> Result<(), String> {
    let mut cb = Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {e}"))?;
    let data = ImageData {
        width,
        height,
        bytes: Cow::Borrowed(pixels),
    };
    cb.set_image(data).map_err(|e| format!("复制图片失败: {e}"))
}
