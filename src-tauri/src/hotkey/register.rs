use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::RwLock;
use std::thread;

use once_cell::sync::Lazy;
use tauri::{AppHandle, Manager};

use super::keybinding::Keybinding;
use crate::wal::logger::WalLogger;

const DOUBLE_CLICK_TIMEOUT_MS: u128 = 500;
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_SECS: u64 = 3;

#[derive(Debug)]
pub enum HotkeyAction {
    Snap,
    GifRecord,
    VideoRecord,
    ForceQuit,
}

/// 全局快捷键绑定映射：action_name -> Keybinding
static BINDINGS: Lazy<RwLock<HashMap<String, Keybinding>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// 从配置加载绑定到全局状态
pub fn load_bindings(hotkeys: &crate::config::HotkeyConfig) {
    let defaults = crate::config::HotkeyConfig::default();
    let mut map = HashMap::new();

    let snap = Keybinding::parse(&hotkeys.snap)
        .unwrap_or_else(|_| Keybinding::parse(&defaults.snap).unwrap());
    map.insert("snap".to_string(), snap);

    let gif = Keybinding::parse(&hotkeys.gif)
        .unwrap_or_else(|_| Keybinding::parse(&defaults.gif).unwrap());
    map.insert("gif".to_string(), gif);

    let video = Keybinding::parse(&hotkeys.video)
        .unwrap_or_else(|_| Keybinding::parse(&defaults.video).unwrap());
    map.insert("video".to_string(), video);

    let force_quit = Keybinding::parse(&hotkeys.force_quit)
        .unwrap_or_else(|_| Keybinding::parse(&defaults.force_quit).unwrap());
    map.insert("force_quit".to_string(), force_quit);

    let mut bindings = BINDINGS.write().unwrap();
    *bindings = map;
}

/// 热重载：重新从配置加载快捷键绑定
pub fn reload_bindings(hotkeys: &crate::config::HotkeyConfig) {
    load_bindings(hotkeys);
}

/// action 名称转枚举
fn action_from_name(name: &str) -> HotkeyAction {
    match name {
        "snap" => HotkeyAction::Snap,
        "gif" => HotkeyAction::GifRecord,
        "video" => HotkeyAction::VideoRecord,
        "force_quit" => HotkeyAction::ForceQuit,
        _ => HotkeyAction::Snap,
    }
}

/// 启动全局快捷键监听。
///
/// 初始化时从 ConfigManager 读取 hotkeys 配置并解析为 Keybinding，
/// 存入全局 BINDINGS。监听线程每次按键事件时读取最新绑定进行匹配。
///
/// Windows/Linux: 使用 rdev
/// macOS: 使用原生 CGEventTap（避免 macOS 15+ TSM 崩溃）
pub fn start_hotkey_listener(app: AppHandle) {
    #[cfg(target_os = "macos")]
    {
        prompt_accessibility_permission();
    }

    // 从配置加载快捷键绑定
    let config_manager = app.state::<crate::config::ConfigManager>();
    let hotkeys = {
        let config = config_manager.config.lock().unwrap();
        config.hotkeys.clone()
    };
    load_bindings(&hotkeys);

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
                HotkeyAction::VideoRecord => {
                    wal.info("HOTKEY", "触发视频录制");
                    let _ = crate::commands::video_cmd::open_video_overlay(&app);
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
    use std::cell::{Cell, RefCell};
    use std::time::Instant;

    use super::keybinding::rdev_key_to_name;

    thread::spawn(move || {
        let mut retries = 0u32;

        loop {
            let tx_clone = tx.clone();
            let result = std::panic::catch_unwind(move || {
                let ctrl_held: Cell<bool> = Cell::new(false);
                let shift_held: Cell<bool> = Cell::new(false);
                let alt_held: Cell<bool> = Cell::new(false);
                let last_double: RefCell<Option<(String, Instant)>> = RefCell::new(None);

                listen(move |event: Event| {
                    match event.event_type {
                        EventType::KeyPress(Key::ControlLeft)
                        | EventType::KeyPress(Key::ControlRight) => ctrl_held.set(true),
                        EventType::KeyRelease(Key::ControlLeft)
                        | EventType::KeyRelease(Key::ControlRight) => ctrl_held.set(false),
                        EventType::KeyPress(Key::ShiftLeft)
                        | EventType::KeyPress(Key::ShiftRight) => shift_held.set(true),
                        EventType::KeyRelease(Key::ShiftLeft)
                        | EventType::KeyRelease(Key::ShiftRight) => shift_held.set(false),
                        EventType::KeyPress(Key::Alt) | EventType::KeyPress(Key::AltGr) => {
                            alt_held.set(true)
                        }
                        EventType::KeyRelease(Key::Alt) | EventType::KeyRelease(Key::AltGr) => {
                            alt_held.set(false)
                        }
                        EventType::KeyPress(ref key) => {
                            if let Some(key_name) = rdev_key_to_name(key) {
                                let bindings = BINDINGS.read().unwrap();
                                for (action, binding) in bindings.iter() {
                                    if binding.matches_rdev(
                                        &key_name,
                                        ctrl_held.get(),
                                        shift_held.get(),
                                        alt_held.get(),
                                    ) {
                                        if binding.double {
                                            // 连击检测：DOUBLE_CLICK_TIMEOUT_MS 内同一 action 连续触发两次
                                            let now = Instant::now();
                                            let should_fire = {
                                                let guard = last_double.borrow();
                                                if let Some((ref prev_action, prev_time)) = *guard
                                                {
                                                    prev_action == action
                                                        && now
                                                            .duration_since(prev_time)
                                                            .as_millis()
                                                            < DOUBLE_CLICK_TIMEOUT_MS
                                                } else {
                                                    false
                                                }
                                            };
                                            if should_fire {
                                                let _ = tx_clone.send(action_from_name(action));
                                                *last_double.borrow_mut() = None;
                                                return;
                                            }
                                            *last_double.borrow_mut() =
                                                Some((action.clone(), now));
                                        } else {
                                            let _ = tx_clone.send(action_from_name(action));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .ok();
            });

            if result.is_ok() {
                break;
            }
            retries += 1;
            eprintln!("[sip-cc] 快捷键监听崩溃，第 {retries} 次重试");
            if retries >= MAX_RETRIES {
                eprintln!("[sip-cc] 快捷键监听重试耗尽，请通过托盘菜单操作");
                return;
            }
            thread::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
        }
    });
}

// ========== macOS: 使用原生 CGEventTap ==========
// 直接读取 keycode + modifier flags，不调用 HIToolbox 字符映射，避免 macOS 15+ 崩溃

#[cfg(target_os = "macos")]
fn start_macos_listener(tx: mpsc::Sender<HotkeyAction>) {
    use std::sync::Mutex;
    use std::time::Instant;

    use super::keybinding::cg_keycode_to_name;

    thread::spawn(move || {
        let mut retries = 0u32;

        loop {
            let tx_clone = tx.clone();
            let result = std::panic::catch_unwind(move || {
                use core_foundation::runloop::*;
                use core_graphics::event::*;

                let last_double: Mutex<Option<(String, Instant)>> = Mutex::new(None);
                let tx = Mutex::new(tx_clone);

                let tap = CGEventTap::new(
                    CGEventTapLocation::Session,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::Default,
                    vec![CGEventType::KeyDown],
                    move |_proxy, _event_type, event| {
                        let keycode =
                            event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                        let flags = event.get_flags();
                        let cmd = flags.contains(CGEventFlags::CGEventFlagCommand);
                        let shift = flags.contains(CGEventFlags::CGEventFlagShift);
                        let alt = flags.contains(CGEventFlags::CGEventFlagAlternate);

                        if let Some(key_name) = cg_keycode_to_name(keycode) {
                            let bindings = BINDINGS.read().unwrap();
                            let tx = tx.lock().unwrap();

                            for (action, binding) in bindings.iter() {
                                if binding.matches_cg(&key_name, cmd, shift, alt) {
                                    if binding.double {
                                        let now = Instant::now();
                                        let mut guard = last_double.lock().unwrap();
                                        if let Some((ref prev_action, prev_time)) = *guard {
                                            if prev_action == action
                                                && now.duration_since(prev_time).as_millis() < DOUBLE_CLICK_TIMEOUT_MS
                                            {
                                                let _ = tx.send(action_from_name(action));
                                                *guard = None;
                                                return CallbackResult::Keep;
                                            }
                                        }
                                        *guard = Some((action.clone(), now));
                                    } else {
                                        let _ = tx.send(action_from_name(action));
                                        return CallbackResult::Drop;
                                    }
                                }
                            }
                        }

                        CallbackResult::Keep
                    },
                );

                match tap {
                    Ok(tap) => {
                        let source = tap
                            .mach_port()
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

            if result.is_ok() {
                break;
            }
            retries += 1;
            eprintln!("[sip-cc] 快捷键监听崩溃，第 {retries} 次重试");
            if retries >= MAX_RETRIES {
                eprintln!("[sip-cc] 快捷键监听重试耗尽，请通过托盘菜单操作");
                return;
            }
            thread::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
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

    match Command::new("osascript")
        .args(["-l", "AppleScript", "-e", script])
        .output()
    {
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
