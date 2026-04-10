use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::snap::capture;

const STATE_RECORDING: u8 = 0;
const STATE_PAUSED: u8 = 1;
const STATE_STOPPED: u8 = 2;
const PAUSE_SLEEP_MS: u64 = 50;

pub struct VideoRecordingSession {
    state: Arc<AtomicU8>,
    handle: Option<thread::JoinHandle<Result<(PathBuf, u32), String>>>,
}

impl VideoRecordingSession {
    pub fn start(
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        fps: u16,
        max_duration_secs: u64,
        tmp_dir: PathBuf,
    ) -> Result<Self, String> {
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("创建临时目录失败: {e}"))?;

        let state = Arc::new(AtomicU8::new(STATE_RECORDING));
        let state_clone = state.clone();

        let handle = thread::spawn(move || {
            let frame_interval = Duration::from_millis(1000 / fps as u64);
            let max_duration = Duration::from_secs(max_duration_secs);
            let start_time = Instant::now();
            let mut frame_count: u32 = 0;

            loop {
                let current_state = state_clone.load(Ordering::Relaxed);
                if current_state == STATE_STOPPED {
                    break;
                }
                if start_time.elapsed() >= max_duration {
                    break;
                }

                if current_state == STATE_PAUSED {
                    thread::sleep(Duration::from_millis(PAUSE_SLEEP_MS));
                    continue;
                }

                let frame_start = Instant::now();
                frame_count += 1;

                match capture::capture_region(x, y, width, height) {
                    Ok(image) => {
                        let frame_path = tmp_dir.join(format!("frame_{:06}.png", frame_count));
                        if let Err(e) = image.save(&frame_path) {
                            eprintln!("视频帧保存失败: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("视频截屏失败: {e}");
                        frame_count -= 1;
                        continue;
                    }
                }

                let elapsed = frame_start.elapsed();
                if elapsed < frame_interval {
                    thread::sleep(frame_interval - elapsed);
                }
            }

            Ok((tmp_dir, frame_count))
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

    pub fn stop(&mut self) -> Result<(PathBuf, u32), String> {
        self.state.store(STATE_STOPPED, Ordering::Relaxed);
        self.handle
            .take()
            .ok_or_else(|| "录制已停止".to_string())?
            .join()
            .map_err(|e| format!("录制线程崩溃: {:?}", e))?
    }
}
