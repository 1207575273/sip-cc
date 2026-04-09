#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// 统一的快捷键绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    pub ctrl: bool,     // macOS 上映射为 Cmd
    pub shift: bool,
    pub alt: bool,      // macOS 上映射为 Option
    pub key: String,    // 主键名：F1, F3, KeyC, Num1 等
    pub double: bool,   // 连击（如 Ctrl+C,Ctrl+C）
}

/// 合法主键白名单
const VALID_KEYS: &[&str] = &[
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyJ",
    "KeyK", "KeyL", "KeyM", "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR", "KeyS", "KeyT",
    "KeyU", "KeyV", "KeyW", "KeyX", "KeyY", "KeyZ",
    "Num0", "Num1", "Num2", "Num3", "Num4", "Num5", "Num6", "Num7", "Num8", "Num9",
    "Backquote", "Minus", "Equal", "BracketLeft", "BracketRight", "Backslash",
    "Semicolon", "Quote", "Comma", "Period", "Slash",
    "Space", "Enter", "Escape", "Backspace", "Tab",
    "PrintScreen", "ScrollLock", "Pause", "Insert", "Delete", "Home", "End", "PageUp", "PageDown",
];

impl Keybinding {
    /// 从配置字符串解析，如 "Ctrl+Shift+S" 或 "F1" 或 "Ctrl+C,Ctrl+C"
    pub fn parse(s: &str) -> Result<Self, String> {
        // 连击检测：逗号分隔
        let double = s.contains(',');
        let part = if double {
            // 取第一段解析（两段应相同）
            s.split(',').next().unwrap_or(s).trim()
        } else {
            s.trim()
        };

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut key = String::new();

        for token in part.split('+') {
            let token = token.trim();
            match token {
                "Ctrl" => ctrl = true,
                "Shift" => shift = true,
                "Alt" => alt = true,
                _ => {
                    key = normalize_key_name(token);
                }
            }
        }

        if key.is_empty() {
            return Err(format!("快捷键缺少主键: {s}"));
        }

        if !VALID_KEYS.contains(&key.as_str()) {
            return Err(format!("不支持的主键: {key}"));
        }

        Ok(Self { ctrl, shift, alt, key, double })
    }

    /// 序列化回配置字符串
    pub fn to_config_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl".to_string()); }
        if self.shift { parts.push("Shift".to_string()); }
        if self.alt { parts.push("Alt".to_string()); }
        parts.push(display_key_name(&self.key));
        let single = parts.join("+");
        if self.double {
            format!("{single},{single}")
        } else {
            single
        }
    }

    /// 用于冲突检测的唯一标识
    pub fn identity(&self) -> String {
        self.to_config_string()
    }

    // ========== 平台匹配 ==========

    /// Windows/Linux: 匹配 rdev Key
    #[cfg(not(target_os = "macos"))]
    pub fn matches_rdev(&self, key_name: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.ctrl == ctrl && self.shift == shift && self.alt == alt && self.key == key_name
    }

    /// macOS: 匹配 CGEventTap keycode + flags
    /// 注意：config 中的 Ctrl 在 macOS 上映射为 Cmd
    #[cfg(target_os = "macos")]
    pub fn matches_cg(&self, key_name: &str, cmd: bool, shift: bool, alt: bool) -> bool {
        self.ctrl == cmd && self.shift == shift && self.alt == alt && self.key == key_name
    }
}

impl fmt::Display for Keybinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_config_string())
    }
}

/// 标准化主键名称（用户输入 -> 内部表示）
fn normalize_key_name(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "a" | "keya" => "KeyA", "b" | "keyb" => "KeyB", "c" | "keyc" => "KeyC",
        "d" | "keyd" => "KeyD", "e" | "keye" => "KeyE", "f" | "keyf" => "KeyF",
        "g" | "keyg" => "KeyG", "h" | "keyh" => "KeyH", "i" | "keyi" => "KeyI",
        "j" | "keyj" => "KeyJ", "k" | "keyk" => "KeyK", "l" | "keyl" => "KeyL",
        "m" | "keym" => "KeyM", "n" | "keyn" => "KeyN", "o" | "keyo" => "KeyO",
        "p" | "keyp" => "KeyP", "q" | "keyq" => "KeyQ", "r" | "keyr" => "KeyR",
        "s" | "keys" => "KeyS", "t" | "keyt" => "KeyT", "u" | "keyu" => "KeyU",
        "v" | "keyv" => "KeyV", "w" | "keyw" => "KeyW", "x" | "keyx" => "KeyX",
        "y" | "keyy" => "KeyY", "z" | "keyz" => "KeyZ",
        "1" | "num1" => "Num1", "2" | "num2" => "Num2", "3" | "num3" => "Num3",
        "4" | "num4" => "Num4", "5" | "num5" => "Num5", "6" | "num6" => "Num6",
        "7" | "num7" => "Num7", "8" | "num8" => "Num8", "9" | "num9" => "Num9",
        "0" | "num0" => "Num0",
        "f1" => "F1", "f2" => "F2", "f3" => "F3", "f4" => "F4",
        "f5" => "F5", "f6" => "F6", "f7" => "F7", "f8" => "F8",
        "f9" => "F9", "f10" => "F10", "f11" => "F11", "f12" => "F12",
        "space" => "Space", "enter" => "Enter", "escape" | "esc" => "Escape",
        "backspace" => "Backspace", "tab" => "Tab", "delete" => "Delete",
        other => other,
    }.to_string()
}

/// 显示主键名称（内部表示 -> 用户友好）
fn display_key_name(name: &str) -> String {
    match name {
        "KeyA" => "A", "KeyB" => "B", "KeyC" => "C", "KeyD" => "D",
        "KeyE" => "E", "KeyF" => "F", "KeyG" => "G", "KeyH" => "H",
        "KeyI" => "I", "KeyJ" => "J", "KeyK" => "K", "KeyL" => "L",
        "KeyM" => "M", "KeyN" => "N", "KeyO" => "O", "KeyP" => "P",
        "KeyQ" => "Q", "KeyR" => "R", "KeyS" => "S", "KeyT" => "T",
        "KeyU" => "U", "KeyV" => "V", "KeyW" => "W", "KeyX" => "X",
        "KeyY" => "Y", "KeyZ" => "Z",
        "Num0" => "0", "Num1" => "1", "Num2" => "2", "Num3" => "3",
        "Num4" => "4", "Num5" => "5", "Num6" => "6", "Num7" => "7",
        "Num8" => "8", "Num9" => "9",
        other => other,
    }.to_string()
}

/// rdev Key 枚举转主键名称（Windows/Linux 用）
#[cfg(not(target_os = "macos"))]
pub fn rdev_key_to_name(key: &rdev::Key) -> Option<String> {
    use rdev::Key;
    let name = match key {
        Key::KeyA => "KeyA", Key::KeyB => "KeyB", Key::KeyC => "KeyC",
        Key::KeyD => "KeyD", Key::KeyE => "KeyE", Key::KeyF => "KeyF",
        Key::KeyG => "KeyG", Key::KeyH => "KeyH", Key::KeyI => "KeyI",
        Key::KeyJ => "KeyJ", Key::KeyK => "KeyK", Key::KeyL => "KeyL",
        Key::KeyM => "KeyM", Key::KeyN => "KeyN", Key::KeyO => "KeyO",
        Key::KeyP => "KeyP", Key::KeyQ => "KeyQ", Key::KeyR => "KeyR",
        Key::KeyS => "KeyS", Key::KeyT => "KeyT", Key::KeyU => "KeyU",
        Key::KeyV => "KeyV", Key::KeyW => "KeyW", Key::KeyX => "KeyX",
        Key::KeyY => "KeyY", Key::KeyZ => "KeyZ",
        Key::Num0 => "Num0", Key::Num1 => "Num1", Key::Num2 => "Num2",
        Key::Num3 => "Num3", Key::Num4 => "Num4", Key::Num5 => "Num5",
        Key::Num6 => "Num6", Key::Num7 => "Num7", Key::Num8 => "Num8",
        Key::Num9 => "Num9",
        Key::F1 => "F1", Key::F2 => "F2", Key::F3 => "F3", Key::F4 => "F4",
        Key::F5 => "F5", Key::F6 => "F6", Key::F7 => "F7", Key::F8 => "F8",
        Key::F9 => "F9", Key::F10 => "F10", Key::F11 => "F11", Key::F12 => "F12",
        Key::Space => "Space", Key::Return => "Enter", Key::Escape => "Escape",
        Key::Backspace => "Backspace", Key::Tab => "Tab", Key::Delete => "Delete",
        _ => return None,
    };
    Some(name.to_string())
}

/// macOS keycode 转主键名称
#[cfg(target_os = "macos")]
pub fn cg_keycode_to_name(keycode: i64) -> Option<String> {
    let name = match keycode {
        0 => "KeyA", 1 => "KeyS", 2 => "KeyD", 3 => "KeyF",
        4 => "KeyH", 5 => "KeyG", 6 => "KeyZ", 7 => "KeyX",
        8 => "KeyC", 9 => "KeyV", 11 => "KeyB", 12 => "KeyQ",
        13 => "KeyW", 14 => "KeyE", 15 => "KeyR", 16 => "KeyY",
        17 => "KeyT",
        // 数字键: 18=1, 19=2, 20=3, 21=4, 23=5, 22=6, 26=7, 28=8, 25=9, 29=0
        18 => "Num1", 19 => "Num2", 20 => "Num3", 21 => "Num4",
        23 => "Num5", 22 => "Num6", 26 => "Num7", 28 => "Num8",
        25 => "Num9", 29 => "Num0",
        // 符号键
        24 => "Equal", 27 => "Minus",
        30 => "BracketRight", 31 => "KeyO", 32 => "KeyU",
        33 => "BracketLeft", 34 => "KeyI", 35 => "KeyP", 36 => "Enter",
        37 => "KeyL", 38 => "KeyJ", 40 => "KeyK", 43 => "Comma",
        44 => "Slash", 45 => "KeyN", 46 => "KeyM", 47 => "Period",
        48 => "Tab", 49 => "Space", 51 => "Backspace", 53 => "Escape",
        117 => "Delete",
        // F keys
        122 => "F1", 120 => "F2", 99 => "F3", 118 => "F4",
        96 => "F5", 97 => "F6", 98 => "F7", 100 => "F8",
        101 => "F9", 109 => "F10", 103 => "F11", 111 => "F12",
        _ => return None,
    };
    Some(name.to_string())
}

/// 检测快捷键冲突，返回冲突的 action 对
pub fn detect_conflicts(bindings: &HashMap<String, Keybinding>) -> Vec<(String, String)> {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut conflicts = Vec::new();

    for (action, binding) in bindings {
        let identity = binding.identity();
        if let Some(existing) = seen.get(&identity) {
            conflicts.push((existing.clone(), action.clone()));
        } else {
            seen.insert(identity, action.clone());
        }
    }

    conflicts
}

/// 系统保留快捷键黑名单
pub fn is_system_reserved(binding: &Keybinding) -> bool {
    let s = binding.to_config_string();
    let reserved = if cfg!(target_os = "macos") {
        vec!["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+A", "Ctrl+Q", "Ctrl+W", "Ctrl+Tab"]
    } else {
        vec!["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+A", "Alt+F4"]
    };
    reserved.contains(&s.as_str())
}
