use rdev::{listen, Event, EventType, Key};
use std::cell::Cell;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use tauri::{AppHandle, Manager};

use crate::wal::logger::WalLogger;

#[derive(Debug)]
pub enum HotkeyAction {
    Snap,
    GifRecord,
    ForceQuit,
}

/// 启动全局快捷键监听。
/// F1 截屏，F3 录制 GIF，双击 Ctrl+C（500ms 内）强制退出。
pub fn start_hotkey_listener(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<HotkeyAction>();

    thread::spawn(move || {
        let last_ctrl_c: Cell<Option<Instant>> = Cell::new(None);
        let ctrl_held: Cell<bool> = Cell::new(false);

        listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                    ctrl_held.set(true);
                }
                EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                    ctrl_held.set(false);
                }
                EventType::KeyPress(Key::KeyC) if ctrl_held.get() => {
                    let now = Instant::now();
                    if let Some(last) = last_ctrl_c.get() {
                        if now.duration_since(last).as_millis() < 500 {
                            let _ = tx.send(HotkeyAction::ForceQuit);
                            last_ctrl_c.set(None);
                            return;
                        }
                    }
                    last_ctrl_c.set(Some(now));
                }
                EventType::KeyPress(Key::F1) => {
                    let _ = tx.send(HotkeyAction::Snap);
                }
                EventType::KeyPress(Key::F3) => {
                    let _ = tx.send(HotkeyAction::GifRecord);
                }
                _ => {}
            }
        })
        .expect("快捷键监听启动失败");
    });

    thread::spawn(move || {
        while let Ok(action) = rx.recv() {
            let wal = app.state::<WalLogger>();
            match action {
                HotkeyAction::Snap => {
                    wal.info("HOTKEY", "F1 触发截屏");
                    let _ = crate::commands::snap_cmd::open_snap_overlay(&app);
                }
                HotkeyAction::GifRecord => {
                    wal.info("HOTKEY", "F3 触发 GIF 录制");
                    let _ = crate::commands::gif_cmd::open_gif_overlay(&app);
                }
                HotkeyAction::ForceQuit => {
                    wal.info("HOTKEY", "Ctrl+C 双击强制退出");
                    // 销毁所有窗口后强制退出
                    for label in &["overlay", "record-bar", "record-region"] {
                        if let Some(win) = app.get_webview_window(label) {
                            let _ = win.destroy();
                        }
                    }
                    std::process::exit(0);
                }
            }
        }
    });
}
