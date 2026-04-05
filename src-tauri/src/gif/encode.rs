use gif::{Encoder, Frame, Repeat};
use std::fs::File;
use std::path::PathBuf;

pub struct GifEncoder {
    encoder: Encoder<File>,
    width: u16,
    height: u16,
    frame_delay: u16,
}

impl GifEncoder {
    pub fn new(path: &PathBuf, width: u16, height: u16, fps: u16) -> Result<Self, String> {
        let file = File::create(path).map_err(|e| format!("创建 GIF 文件失败: {e}"))?;
        let mut encoder = Encoder::new(file, width, height, &[])
            .map_err(|e| format!("初始化 GIF 编码器失败: {e}"))?;
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| format!("设置循环播放失败: {e}"))?;

        // GIF delay 单位是 1/100 秒
        let frame_delay = 100 / fps;
        Ok(Self {
            encoder,
            width,
            height,
            frame_delay,
        })
    }

    pub fn add_frame(&mut self, rgba_pixels: &[u8]) -> Result<(), String> {
        let mut pixels = rgba_pixels.to_vec();
        let mut frame = Frame::from_rgba_speed(self.width, self.height, &mut pixels, 10);
        frame.delay = self.frame_delay;
        self.encoder
            .write_frame(&frame)
            .map_err(|e| format!("写入帧失败: {e}"))
    }
}
