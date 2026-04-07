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
///
/// Windows/Linux: F1 截屏，F3 录制 GIF，双击 Ctrl+C 强制退出
/// macOS: Cmd+Shift+1 截屏，Cmd+Shift+3 录制 GIF，双击 Cmd+C 强制退出
///        （macOS F1-F12 默认是系统功能键，需要 fn 配合，体验差）
///        （macOS 需要授予辅助功能 Accessibility 权限才能监听全局键盘）
pub fn start_hotkey_listener(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<HotkeyAction>();

    thread::spawn(move || {
        let ctrl_held: Cell<bool> = Cell::new(false);   // Windows/Linux: Ctrl
        let meta_held: Cell<bool> = Cell::new(false);    // macOS: Cmd (MetaLeft/MetaRight)
        let shift_held: Cell<bool> = Cell::new(false);
        let last_quit_combo: Cell<Option<Instant>> = Cell::new(None);

        listen(move |event: Event| {
            match event.event_type {
                // Ctrl 键（Windows/Linux 的修饰键）
                EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                    ctrl_held.set(true);
                }
                EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                    ctrl_held.set(false);
                }
                // Cmd 键（macOS 的修饰键）
                EventType::KeyPress(Key::MetaLeft) | EventType::KeyPress(Key::MetaRight) => {
                    meta_held.set(true);
                }
                EventType::KeyRelease(Key::MetaLeft) | EventType::KeyRelease(Key::MetaRight) => {
                    meta_held.set(false);
                }
                // Shift 键
                EventType::KeyPress(Key::ShiftLeft) | EventType::KeyPress(Key::ShiftRight) => {
                    shift_held.set(true);
                }
                EventType::KeyRelease(Key::ShiftLeft) | EventType::KeyRelease(Key::ShiftRight) => {
                    shift_held.set(false);
                }

                // === 截屏快捷键 ===
                // Windows/Linux: F1
                EventType::KeyPress(Key::F1) if !meta_held.get() => {
                    let _ = tx.send(HotkeyAction::Snap);
                }
                // macOS: Cmd+Shift+1
                EventType::KeyPress(Key::Num1) if meta_held.get() && shift_held.get() => {
                    let _ = tx.send(HotkeyAction::Snap);
                }

                // === GIF 录制快捷键 ===
                // Windows/Linux: F3
                EventType::KeyPress(Key::F3) if !meta_held.get() => {
                    let _ = tx.send(HotkeyAction::GifRecord);
                }
                // macOS: Cmd+Shift+3
                EventType::KeyPress(Key::Num3) if meta_held.get() && shift_held.get() => {
                    let _ = tx.send(HotkeyAction::GifRecord);
                }

                // === 强制退出 ===
                // Windows/Linux: 双击 Ctrl+C
                // macOS: 双击 Cmd+C
                EventType::KeyPress(Key::KeyC) if ctrl_held.get() || meta_held.get() => {
                    let now = Instant::now();
                    if let Some(last) = last_quit_combo.get() {
                        if now.duration_since(last).as_millis() < 500 {
                            let _ = tx.send(HotkeyAction::ForceQuit);
                            last_quit_combo.set(None);
                            return;
                        }
                    }
                    last_quit_combo.set(Some(now));
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
                    wal.info("HOTKEY", "触发截屏");
                    let _ = crate::commands::snap_cmd::open_snap_overlay(&app);
                }
                HotkeyAction::GifRecord => {
                    wal.info("HOTKEY", "触发 GIF 录制");
                    let _ = crate::commands::gif_cmd::open_gif_overlay(&app);
                }
                HotkeyAction::ForceQuit => {
                    wal.info("HOTKEY", "双击强制退出");
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
