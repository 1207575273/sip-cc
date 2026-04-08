use std::sync::mpsc;
use std::thread;
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
/// Windows/Linux: F1 截屏，F3 录制 GIF，双击 Ctrl+C 强制退出（使用 rdev）
/// macOS: Cmd+Shift+1 截屏，Cmd+Shift+3 录制 GIF，双击 Cmd+C 强制退出（使用原生 CGEventTap）
///
/// macOS 不用 rdev 的原因：rdev::listen 内部调用 Keyboard::string_from_code → TSMGetInputSourceProperty，
/// 该 API 在 macOS 15+ 要求主线程执行（dispatch_assert_queue 断言），在子线程调用直接崩溃 (SIGTRAP)。
/// 改用 CGEventTap 直接读取 keycode，完全绕开 HIToolbox 的字符映射。
pub fn start_hotkey_listener(app: AppHandle) {
    #[cfg(target_os = "macos")]
    {
        prompt_accessibility_permission();
    }

    let (tx, rx) = mpsc::channel::<HotkeyAction>();

    #[cfg(target_os = "macos")]
    start_macos_listener(tx);

    #[cfg(not(target_os = "macos"))]
    start_rdev_listener(tx);

    // 处理线程
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

// ========== Windows/Linux: 使用 rdev ==========

#[cfg(not(target_os = "macos"))]
fn start_rdev_listener(tx: mpsc::Sender<HotkeyAction>) {
    use rdev::{listen, Event, EventType, Key};
    use std::cell::Cell;
    use std::time::Instant;

    thread::spawn(move || {
        let ctrl_held: Cell<bool> = Cell::new(false);
        let last_ctrl_c: Cell<Option<Instant>> = Cell::new(None);

        listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                    ctrl_held.set(true);
                }
                EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                    ctrl_held.set(false);
                }
                EventType::KeyPress(Key::F1) => {
                    let _ = tx.send(HotkeyAction::Snap);
                }
                EventType::KeyPress(Key::F3) => {
                    let _ = tx.send(HotkeyAction::GifRecord);
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
                _ => {}
            }
        })
        .expect("快捷键监听启动失败");
    });
}

// ========== macOS: 使用原生 CGEventTap ==========
// 直接读取 keycode + modifier flags，不调用 HIToolbox 字符映射，避免 macOS 15+ 崩溃

#[cfg(target_os = "macos")]
fn start_macos_listener(tx: mpsc::Sender<HotkeyAction>) {
    use std::sync::Mutex;
    use std::time::Instant;

    thread::spawn(move || {
        use core_graphics::event::*;
        use core_foundation::runloop::*;

        // macOS keycodes（硬件扫描码）
        const KC_1: i64 = 18;  // 数字 1
        const KC_3: i64 = 20;  // 数字 3
        const KC_C: i64 = 8;   // 字母 C

        let last_cmd_c: Mutex<Option<Instant>> = Mutex::new(None);
        let tx = Mutex::new(tx);

        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown],
            move |_proxy, _event_type, event| {
                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                let flags = event.get_flags();
                let cmd = flags.contains(CGEventFlags::CGEventFlagCommand);
                let shift = flags.contains(CGEventFlags::CGEventFlagShift);

                let tx = tx.lock().unwrap();

                // Cmd+Shift+1 → 截屏
                if cmd && shift && keycode == KC_1 {
                    let _ = tx.send(HotkeyAction::Snap);
                    return CallbackResult::Drop;
                }
                // Cmd+Shift+3 → GIF 录制
                if cmd && shift && keycode == KC_3 {
                    let _ = tx.send(HotkeyAction::GifRecord);
                    return CallbackResult::Drop;
                }
                // Cmd+C 双击 → 强制退出
                if cmd && keycode == KC_C {
                    let now = Instant::now();
                    let mut guard = last_cmd_c.lock().unwrap();
                    if let Some(last) = *guard {
                        if now.duration_since(last).as_millis() < 500 {
                            let _ = tx.send(HotkeyAction::ForceQuit);
                            *guard = None;
                            return CallbackResult::Keep;
                        }
                    }
                    *guard = Some(now);
                }

                CallbackResult::Keep
            },
        );

        match tap {
            Ok(tap) => {
                let source = tap.mach_port()
                    .create_runloop_source(0)
                    .expect("RunLoop source 创建失败");

                unsafe {
                    let run_loop = CFRunLoop::get_current();
                    run_loop.add_source(&source, kCFRunLoopCommonModes);
                }
                tap.enable();
                CFRunLoop::run_current();
            }
            Err(()) => {
                eprintln!("[sip-cc] CGEventTap 创建失败，需要辅助功能权限");
            }
        }
    });
}

/// macOS: 检查辅助功能权限，未授权时触发系统弹窗引导
#[cfg(target_os = "macos")]
fn prompt_accessibility_permission() {
    use std::process::Command;

    let script = r#"
        use framework "Foundation"
        use framework "ApplicationServices"

        set options to current application's NSDictionary's dictionaryWithObject:true forKey:"AXTrustedCheckOptionPrompt"
        set trusted to current application's AXIsProcessTrustedWithOptions(options)
        return trusted as boolean
    "#;

    match Command::new("osascript").args(["-l", "AppleScript", "-e", script]).output() {
        Ok(output) => {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if result == "true" {
                eprintln!("[sip-cc] macOS 辅助功能权限: 已授权");
            } else {
                eprintln!("[sip-cc] macOS 辅助功能权限: 未授权，已弹出系统授权引导");
            }
        }
        Err(e) => {
            eprintln!("[sip-cc] 辅助功能权限检查失败: {e}");
        }
    }
}
