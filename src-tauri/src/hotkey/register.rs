use rdev::{grab, Event, EventType, Key};
use std::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Manager};

use crate::wal::logger::WalLogger;

#[derive(Debug)]
pub enum HotkeyAction {
    Snap,
    GifRecord,
}

/// 启动全局快捷键监听，使用 rdev::grab 强制抢占 F1/F3。
/// 被抢占的按键不会传递给其他程序。
pub fn start_hotkey_listener(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<HotkeyAction>();

    // grab 线程：拦截 F1/F3，吞掉事件不传递给其他程序
    thread::spawn(move || {
        grab(move |event: Event| -> Option<Event> {
            match event.event_type {
                EventType::KeyPress(Key::F1) => {
                    let _ = tx.send(HotkeyAction::Snap);
                    None // 强制抢占：吞掉事件
                }
                EventType::KeyPress(Key::F3) => {
                    let _ = tx.send(HotkeyAction::GifRecord);
                    None
                }
                _ => Some(event), // 其他键正常传递
            }
        })
        .expect("快捷键监听启动失败");
    });

    // 处理线程：接收快捷键事件并执行对应操作
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
            }
        }
    });
}
