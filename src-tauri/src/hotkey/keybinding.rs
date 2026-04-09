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

/// 系统保留快捷键黑名单（Windows 视角，macOS 上 Ctrl 映射为 Cmd）
pub fn is_system_reserved(binding: &Keybinding) -> bool {
    let s = binding.to_config_string();
    let reserved = if cfg!(target_os = "macos") {
        vec!["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+A", "Ctrl+Q", "Ctrl+W", "Ctrl+Tab"]
    } else {
        vec!["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+A", "Alt+F4"]
    };
    reserved.contains(&s.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Keybinding::parse 解析测试 ==========

    #[test]
    fn should_parse_single_key() {
        let kb = Keybinding::parse("F1").unwrap();
        assert!(!kb.ctrl);
        assert!(!kb.shift);
        assert!(!kb.alt);
        assert_eq!(kb.key, "F1");
        assert!(!kb.double);
    }

    #[test]
    fn should_parse_modifier_plus_key() {
        let kb = Keybinding::parse("Ctrl+Shift+S").unwrap();
        assert!(kb.ctrl);
        assert!(kb.shift);
        assert!(!kb.alt);
        assert_eq!(kb.key, "KeyS");
        assert!(!kb.double);
    }

    #[test]
    fn should_parse_double_click_combo() {
        let kb = Keybinding::parse("Ctrl+C,Ctrl+C").unwrap();
        assert!(kb.ctrl);
        assert!(!kb.shift);
        assert_eq!(kb.key, "KeyC");
        assert!(kb.double);
    }

    #[test]
    fn should_parse_alt_modifier() {
        let kb = Keybinding::parse("Alt+F4").unwrap();
        assert!(!kb.ctrl);
        assert!(!kb.shift);
        assert!(kb.alt);
        assert_eq!(kb.key, "F4");
    }

    #[test]
    fn should_parse_number_key() {
        let kb = Keybinding::parse("Ctrl+Shift+1").unwrap();
        assert!(kb.ctrl);
        assert!(kb.shift);
        assert_eq!(kb.key, "Num1");
    }

    #[test]
    fn should_fail_on_empty_string() {
        assert!(Keybinding::parse("").is_err());
    }

    #[test]
    fn should_fail_on_only_modifiers() {
        assert!(Keybinding::parse("Ctrl+Shift").is_err());
    }

    #[test]
    fn should_fail_on_invalid_key() {
        assert!(Keybinding::parse("Ctrl+XYZ123").is_err());
    }

    #[test]
    fn should_trim_whitespace() {
        let kb = Keybinding::parse("  F3  ").unwrap();
        assert_eq!(kb.key, "F3");
    }

    // ========== to_config_string 序列化测试 ==========

    #[test]
    fn should_serialize_single_key() {
        let kb = Keybinding::parse("F1").unwrap();
        assert_eq!(kb.to_config_string(), "F1");
    }

    #[test]
    fn should_serialize_combo_key() {
        let kb = Keybinding::parse("Ctrl+Shift+S").unwrap();
        assert_eq!(kb.to_config_string(), "Ctrl+Shift+S");
    }

    #[test]
    fn should_serialize_double_click() {
        let kb = Keybinding::parse("Ctrl+C,Ctrl+C").unwrap();
        assert_eq!(kb.to_config_string(), "Ctrl+C,Ctrl+C");
    }

    #[test]
    fn should_roundtrip_parse_and_serialize() {
        let cases = vec!["F1", "F12", "Ctrl+Shift+1", "Alt+F4", "Ctrl+C,Ctrl+C", "Ctrl+Shift+3"];
        for input in cases {
            let kb = Keybinding::parse(input).unwrap();
            assert_eq!(kb.to_config_string(), input, "roundtrip failed for: {input}");
        }
    }

    // ========== normalize_key_name 标准化测试 ==========

    #[test]
    fn should_normalize_lowercase_letter() {
        assert_eq!(normalize_key_name("a"), "KeyA");
        assert_eq!(normalize_key_name("z"), "KeyZ");
    }

    #[test]
    fn should_normalize_number() {
        assert_eq!(normalize_key_name("1"), "Num1");
        assert_eq!(normalize_key_name("0"), "Num0");
    }

    #[test]
    fn should_normalize_fkey_case_insensitive() {
        assert_eq!(normalize_key_name("f1"), "F1");
        assert_eq!(normalize_key_name("f12"), "F12");
    }

    #[test]
    fn should_pass_through_unknown() {
        assert_eq!(normalize_key_name("Space"), "Space");
    }

    // ========== display_key_name 显示名称测试 ==========

    #[test]
    fn should_display_key_letter() {
        assert_eq!(display_key_name("KeyA"), "A");
        assert_eq!(display_key_name("KeyZ"), "Z");
    }

    #[test]
    fn should_display_key_number() {
        assert_eq!(display_key_name("Num0"), "0");
        assert_eq!(display_key_name("Num9"), "9");
    }

    #[test]
    fn should_display_fkey_as_is() {
        assert_eq!(display_key_name("F1"), "F1");
        assert_eq!(display_key_name("Space"), "Space");
    }

    // ========== 冲突检测测试 ==========

    #[test]
    fn should_detect_no_conflict() {
        let mut map = HashMap::new();
        map.insert("snap".to_string(), Keybinding::parse("F1").unwrap());
        map.insert("gif".to_string(), Keybinding::parse("F3").unwrap());
        let conflicts = detect_conflicts(&map);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn should_detect_conflict_same_key() {
        let mut map = HashMap::new();
        map.insert("snap".to_string(), Keybinding::parse("F1").unwrap());
        map.insert("gif".to_string(), Keybinding::parse("F1").unwrap());
        let conflicts = detect_conflicts(&map);
        assert_eq!(conflicts.len(), 1);
    }

    // ========== 系统保留键测试 ==========

    #[test]
    fn should_detect_ctrl_c_as_reserved() {
        let kb = Keybinding::parse("Ctrl+C").unwrap();
        assert!(is_system_reserved(&kb));
    }

    #[test]
    fn should_not_detect_f1_as_reserved() {
        let kb = Keybinding::parse("F1").unwrap();
        assert!(!is_system_reserved(&kb));
    }

    #[test]
    fn should_detect_ctrl_v_as_reserved() {
        let kb = Keybinding::parse("Ctrl+V").unwrap();
        assert!(is_system_reserved(&kb));
    }

    // ========== 平台匹配测试（仅 Windows/Linux） ==========

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn should_match_rdev_f1() {
        let kb = Keybinding::parse("F1").unwrap();
        assert!(kb.matches_rdev("F1", false, false, false));
        assert!(!kb.matches_rdev("F1", true, false, false));  // Ctrl 按下时不匹配
        assert!(!kb.matches_rdev("F2", false, false, false));  // 不同键不匹配
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn should_match_rdev_combo() {
        let kb = Keybinding::parse("Ctrl+Shift+S").unwrap();
        assert!(kb.matches_rdev("KeyS", true, true, false));
        assert!(!kb.matches_rdev("KeyS", true, false, false));  // 缺 Shift
        assert!(!kb.matches_rdev("KeyA", true, true, false));    // 不同键
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn should_convert_rdev_key_to_name() {
        use rdev::Key;
        assert_eq!(rdev_key_to_name(&Key::F1), Some("F1".to_string()));
        assert_eq!(rdev_key_to_name(&Key::KeyA), Some("KeyA".to_string()));
        assert_eq!(rdev_key_to_name(&Key::Num0), Some("Num0".to_string()));
        assert_eq!(rdev_key_to_name(&Key::Space), Some("Space".to_string()));
        assert_eq!(rdev_key_to_name(&Key::Return), Some("Enter".to_string()));
    }
}
