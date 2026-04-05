use rdev::{listen, Event, EventType, Key};
use std::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Manager};

use crate::wal::logger::WalLogger;

#[derive(Debug)]
pub enum HotkeyAction {
    Snap,
    GifRecord,
}

/// 启动全局快捷键监听，使用 rdev::listen（非阻塞，在独立线程中运行）。
/// 注意：listen 不会拦截按键，只是监听；如需拦截需启用 unstable_grab feature。
pub fn start_hotkey_listener(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<HotkeyAction>();

    // 监听线程：捕获全局键盘事件
    thread::spawn(move || {
        listen(move |event: Event| {
            match event.event_type {
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
