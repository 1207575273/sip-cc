use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::snap::{capture, crop};

use super::encode::GifEncoder;

const STATE_RECORDING: u8 = 0;
const STATE_PAUSED: u8 = 1;
const STATE_STOPPED: u8 = 2;

pub struct RecordingSession {
    state: Arc<AtomicU8>,
    handle: Option<thread::JoinHandle<Result<PathBuf, String>>>,
}

impl RecordingSession {
    pub fn start(
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        fps: u16,
        max_duration_secs: u64,
        output_path: PathBuf,
    ) -> Result<Self, String> {
        let state = Arc::new(AtomicU8::new(STATE_RECORDING));
        let state_clone = state.clone();
        let path = output_path.clone();

        let handle = thread::spawn(move || {
            let mut encoder = GifEncoder::new(&path, width as u16, height as u16, fps)?;
            let frame_interval = Duration::from_millis(1000 / fps as u64);
            let max_duration = Duration::from_secs(max_duration_secs);
            let start_time = Instant::now();

            loop {
                let current_state = state_clone.load(Ordering::Relaxed);
                if current_state == STATE_STOPPED {
                    break;
                }
                if start_time.elapsed() >= max_duration {
                    break;
                }

                if current_state == STATE_PAUSED {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }

                let frame_start = Instant::now();
                let screen = capture::capture_primary_screen()?;
                let cropped = crop::crop_rgba(&screen.image, x, y, width, height)?;
                encoder.add_frame(cropped.as_raw())?;

                let elapsed = frame_start.elapsed();
                if elapsed < frame_interval {
                    thread::sleep(frame_interval - elapsed);
                }
            }

            Ok(path)
        });

        Ok(Self {
            state,
            handle: Some(handle),
        })
    }

    pub fn pause(&self) {
        self.state.store(STATE_PAUSED, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.state.store(STATE_RECORDING, Ordering::Relaxed);
    }

    pub fn stop(&mut self) -> Result<PathBuf, String> {
        self.state.store(STATE_STOPPED, Ordering::Relaxed);
        self.handle
            .take()
            .ok_or_else(|| "录制已停止".to_string())?
            .join()
            .map_err(|_| "录制线程崩溃".to_string())?
    }
}
