//! 长截图 Tauri 命令（第一阶段仅 Windows）：**滚轮触发 + 画面变化** 追加帧 + 拼接

use std::sync::Mutex;
use std::time::Duration;

use image::RgbaImage;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
use once_cell::sync::Lazy;
#[cfg(windows)]
use std::sync::mpsc;
#[cfg(windows)]
use std::thread;

use crate::commands::snap_cmd::Region;

/// 进行中的手动长截图会话
pub struct ManualSession {
    pub region: Region,
    pub frames: Vec<RgbaImage>,
}

pub struct LongSnapManualState(pub Mutex<Option<ManualSession>>);

/// 全局滚轮 → 工作线程：防抖后截屏，与上一帧平均差低于阈值则丢弃（避免重复帧）
#[cfg(windows)]
static LONG_SNAP_WHEEL_TX: Lazy<Mutex<Option<mpsc::Sender<()>>>> =
    Lazy::new(|| Mutex::new(None));

/// 滚轮停稳后再截（ms）
#[cfg(windows)]
const LONG_SNAP_SCROLL_DEBOUNCE_MS: u64 = 160;
/// 与上一帧全图平均通道差低于此值视为「无变化」，不追加（可与 `DEFAULT_BOTTOM_MEAN_DIFF` 同量级调参）
#[cfg(windows)]
const LONG_SNAP_MIN_MEAN_DIFF_APPEND: f64 = 2.8;

#[cfg(windows)]
fn clear_long_snap_wheel_channel() {
    if let Ok(mut g) = LONG_SNAP_WHEEL_TX.lock() {
        *g = None;
    }
}

/// 由全局 rdev 在收到鼠标滚轮时调用（仅 Windows 长截图会话中 `LONG_SNAP_WHEEL_TX` 有值时生效）
#[cfg(windows)]
pub fn long_snap_wheel_tick() {
    if let Ok(guard) = LONG_SNAP_WHEEL_TX.lock() {
        if let Some(ref tx) = *guard {
            let _ = tx.send(());
        }
    }
}

#[cfg(not(windows))]
pub fn long_snap_wheel_tick() {}

#[cfg(windows)]
fn wheel_worker(app: AppHandle, rx: mpsc::Receiver<()>) {
    let debounce = Duration::from_millis(LONG_SNAP_SCROLL_DEBOUNCE_MS);
    while let Ok(()) = rx.recv() {
        while rx.try_recv().is_ok() {}
        thread::sleep(debounce);
        let _ = try_append_after_scroll(&app);
    }
}

/// 滚轮防抖后：有会话则截屏；若已有帧且与上一帧差异过小则丢弃
#[cfg(windows)]
fn try_append_after_scroll(app: &AppHandle) -> Result<(), String> {
    let st = app.state::<LongSnapManualState>();
    let region = {
        let g = st.0.lock().map_err(|e| e.to_string())?;
        let Some(session) = g.as_ref() else {
            return Ok(());
        };
        session.region.clone()
    };

    let mut new_frame = crate::snap::long_snap_win::capture_long_snap_frame(app, &region)?;

    let mut g = st.0.lock().map_err(|e| e.to_string())?;
    let Some(session) = g.as_mut() else {
        return Ok(());
    };
    if session.frames.len() >= crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES {
        return Ok(());
    }

    if let Some(last) = session.frames.last() {
        let diff = crate::snap::long_snap::mean_abs_diff_excluding_rect(
            last,
            &new_frame,
            0,
            0,
            crate::snap::frame_index_stamp::EXCLUDE_W,
            crate::snap::frame_index_stamp::EXCLUDE_H,
        );
        if diff < LONG_SNAP_MIN_MEAN_DIFF_APPEND {
            return Ok(());
        }
    }

    let next_idx = session.frames.len() as u32 + 1;
    crate::snap::frame_index_stamp::stamp_frame_index_top_left(&mut new_frame, next_idx);
    session.frames.push(new_frame);
    let n = session.frames.len() as u32;
    drop(g);
    let _ = app.emit(
        "long-snap-progress",
        serde_json::json!({
            "current": n,
            "max": crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES
        }),
    );
    Ok(())
}

/// 进入模式后首帧：仅当仍无帧时写入（与滚轮线程竞态时用锁保证只追加一次）
#[cfg(windows)]
fn try_append_first_frame_if_empty(app: &AppHandle) -> Result<(), String> {
    let st = app.state::<LongSnapManualState>();
    let region = {
        let g = st.0.lock().map_err(|e| e.to_string())?;
        let Some(session) = g.as_ref() else {
            return Ok(());
        };
        if !session.frames.is_empty() {
            return Ok(());
        }
        session.region.clone()
    };

    let mut frame = crate::snap::long_snap_win::capture_long_snap_frame(app, &region)?;

    let mut g = st.0.lock().map_err(|e| e.to_string())?;
    let Some(session) = g.as_mut() else {
        return Ok(());
    };
    if !session.frames.is_empty() {
        return Ok(());
    }
    if session.frames.len() >= crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES {
        return Ok(());
    }
    crate::snap::frame_index_stamp::stamp_frame_index_top_left(&mut frame, 1);
    session.frames.push(frame);
    let n = session.frames.len() as u32;
    drop(g);
    let _ = app.emit(
        "long-snap-progress",
        serde_json::json!({
            "current": n,
            "max": crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES
        }),
    );
    Ok(())
}

/// overlay 隐藏时清会话并关「完成/取消」小窗；若仍有会话则发 `long-snap-cancelled` 供主界面收尾
pub fn reset_long_snap_on_overlay_hide(app: &AppHandle) {
    #[cfg(windows)]
    {
        clear_long_snap_wheel_channel();
        let mut had_session = false;
        if let Some(st) = app.try_state::<LongSnapManualState>() {
            if let Ok(mut g) = st.0.lock() {
                had_session = g.is_some();
                *g = None;
            }
        }
        crate::snap::long_snap_win::close_long_snap_control_window(app);
        if had_session {
            let _ = app.emit("long-snap-cancelled", serde_json::json!({}));
        }
    }
}

#[tauri::command]
pub fn long_snap_supported() -> bool {
    cfg!(windows)
}

/// 开始：建立会话、打开浮动控制条、启动滚轮采样线程并补首帧
#[tauri::command]
pub fn long_snap_start(app: AppHandle, region: Region) -> Result<(), String> {
    #[cfg(windows)]
    {
        clear_long_snap_wheel_channel();

        let region_for_win = region.clone();
        {
            let st = app.state::<LongSnapManualState>();
            let mut g = st.0.lock().map_err(|e| e.to_string())?;
            *g = Some(ManualSession {
                region,
                frames: vec![],
            });
        }
        let _ = app.emit(
            "long-snap-progress",
            serde_json::json!({
                "current": 0,
                "max": crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES
            }),
        );

        let (tx, rx) = mpsc::channel();
        {
            let mut guard = LONG_SNAP_WHEEL_TX.lock().map_err(|e| e.to_string())?;
            *guard = Some(tx);
        }
        let app_wheel = app.clone();
        thread::spawn(move || {
            wheel_worker(app_wheel, rx);
        });

        let app_first = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(280));
            let _ = try_append_first_frame_if_empty(&app_first);
        });

        let app_win = app.clone();
        thread::spawn(move || {
            let _ = crate::snap::long_snap_win::open_long_snap_control_window(&app_win, &region_for_win);
        });
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (app, region);
        Err("长截图仅支持 Windows".to_string())
    }
}

/// 兼容：强制追加一帧（不比较差异）；前端当前不再定时调用
#[tauri::command]
pub fn long_snap_append_frame(app: AppHandle) -> Result<u32, String> {
    #[cfg(windows)]
    {
        let st = app.state::<LongSnapManualState>();
        let mut g = st.0.lock().map_err(|e| e.to_string())?;
        let session = g.as_mut().ok_or("没有进行中的长截图")?;
        if session.frames.len() >= crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES {
            return Err(format!(
                "已达到最大帧数（{}）",
                crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES
            ));
        }
        let region = session.region.clone();
        let mut frame = crate::snap::long_snap_win::capture_long_snap_frame(&app, &region)?;
        let idx = session.frames.len() as u32 + 1;
        crate::snap::frame_index_stamp::stamp_frame_index_top_left(&mut frame, idx);
        session.frames.push(frame);
        let n = session.frames.len() as u32;
        let _ = app.emit(
            "long-snap-progress",
            serde_json::json!({
                "current": n,
                "max": crate::snap::long_snap_win::MAX_LONG_SNAP_FRAMES
            }),
        );
        Ok(n)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("长截图仅支持 Windows".to_string())
    }
}

/// 完成拼接并保存
#[tauri::command]
pub fn long_snap_finish(app: AppHandle) -> Result<String, String> {
    #[cfg(windows)]
    {
        clear_long_snap_wheel_channel();

        let st = app.state::<LongSnapManualState>();
        let mut g = st.0.lock().map_err(|e| e.to_string())?;
        let session = g.take().ok_or("没有进行中的长截图")?;
        drop(g);

        let _ = app.emit("long-snap-stitching", serde_json::json!({}));

        let path = match crate::snap::long_snap_win::save_stitched_long_snap(&app, session.frames) {
            Ok(p) => p,
            Err(e) => {
                let _ = app.emit(
                    "long-snap-error",
                    serde_json::json!({ "message": e.clone() }),
                );
                return Err(e);
            }
        };
        crate::snap::long_snap_win::close_long_snap_control_window(&app);
        let _ = app.emit(
            "long-snap-done",
            serde_json::json!({ "path": path.clone() }),
        );
        Ok(path)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("长截图仅支持 Windows".to_string())
    }
}

#[tauri::command]
pub fn long_snap_cancel(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        clear_long_snap_wheel_channel();
        let st = app.state::<LongSnapManualState>();
        let mut g = st.0.lock().map_err(|e| e.to_string())?;
        *g = None;
        crate::snap::long_snap_win::close_long_snap_control_window(&app);
        let _ = app.emit("long-snap-cancelled", serde_json::json!({}));
    }
    Ok(())
}
