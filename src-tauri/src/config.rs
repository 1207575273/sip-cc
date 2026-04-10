use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// 与 `Cargo.toml` / 发布版本一致；配置里存此字段，用于升级时判断是否沿用旧文件。
pub const CONFIG_FILE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SaveDir {
    Desktop,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub snap: String,
    pub gif: String,
    #[serde(default = "default_video_hotkey")]
    pub video: String,
    pub force_quit: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        // 平台感知默认值
        if cfg!(target_os = "macos") {
            Self {
                snap: "Ctrl+Shift+1".to_string(),     // macOS 上 Ctrl 映射为 Cmd
                gif: "Ctrl+Shift+3".to_string(),
                video: "Ctrl+Shift+5".to_string(),
                force_quit: "Ctrl+C,Ctrl+C".to_string(),
            }
        } else {
            Self {
                snap: "F1".to_string(),
                gif: "F3".to_string(),
                video: "F5".to_string(),
                force_quit: "Ctrl+C,Ctrl+C".to_string(),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 写入本文件时的应用版本；与 `CONFIG_FILE_VERSION` 不一致时整表重置为默认（新能力用新默认值）
    #[serde(default)]
    pub version: String,
    /// 保存目录类型：desktop / custom
    pub save_dir: SaveDir,
    /// 自定义保存目录路径（save_dir=custom 时使用）
    pub custom_save_dir: Option<String>,
    /// GIF 录制帧率
    pub gif_fps: u16,
    /// GIF 最大录制时长（秒，1～600，默认 10 分钟）
    #[serde(default = "default_gif_max_duration")]
    pub gif_max_duration_secs: u64,
    /// 快捷键配置
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
    #[serde(default = "default_video_fps")]
    pub video_fps: u16,
    #[serde(default = "default_video_crf")]
    pub video_crf: u8,
    #[serde(default = "default_video_preset")]
    pub video_preset: String,
    #[serde(default = "default_video_max_duration")]
    pub video_max_duration_secs: u64,
    /// 视频编码完成后是否复制文件到剪贴板（默认关闭，避免大体积 MP4 占用剪贴板）
    #[serde(default)]
    pub video_copy_to_clipboard: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_FILE_VERSION.to_string(),
            save_dir: SaveDir::Desktop,
            custom_save_dir: None,
            gif_fps: 10,
            gif_max_duration_secs: 600,
            hotkeys: HotkeyConfig::default(),
            video_fps: 15,
            video_crf: 23,
            video_preset: "medium".to_string(),
            video_max_duration_secs: 1800,
            video_copy_to_clipboard: false,
        }
    }
}

fn default_gif_max_duration() -> u64 {
    600
}

fn default_video_hotkey() -> String {
    if cfg!(target_os = "macos") { "Ctrl+Shift+5".to_string() }
    else { "F5".to_string() }
}

fn default_video_fps() -> u16 { 15 }
fn default_video_crf() -> u8 { 23 }
fn default_video_preset() -> String { "medium".to_string() }
fn default_video_max_duration() -> u64 {
    1800
}

pub struct ConfigManager {
    pub config: Mutex<AppConfig>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = Self::config_file_path();
        let config = Self::load_from_file(&config_path);

        let manager = Self {
            config: Mutex::new(config),
            config_path,
        };

        // 首次启动：如果配置文件不存在，自动初始化写入默认配置
        manager.ensure_config_file();
        manager
    }

    pub fn base_dir() -> PathBuf {
        dirs::home_dir()
            .expect("failed to get home directory")
            .join(".sip-cc")
    }

    fn config_file_path() -> PathBuf {
        Self::base_dir().join("config.json")
    }

    fn load_from_file(path: &PathBuf) -> AppConfig {
        let mut config = match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        };

        if config.version != CONFIG_FILE_VERSION {
            config = AppConfig::default();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(text) = serde_json::to_string_pretty(&config) {
                let _ = fs::write(path, text);
            }
            return config;
        }

        Self::clamp_config_values(&mut config);
        config
    }

    fn clamp_config_values(config: &mut AppConfig) {
        config.gif_fps = config.gif_fps.clamp(1, 60);
        config.gif_max_duration_secs = config.gif_max_duration_secs.clamp(1, 600);
        config.video_fps = config.video_fps.clamp(5, 60);
        config.video_crf = config.video_crf.clamp(0, 51);
        config.video_max_duration_secs = config.video_max_duration_secs.clamp(1, 7200);
        if !["ultrafast", "superfast", "veryfast", "faster", "fast",
             "medium", "slow", "slower", "veryslow"].contains(&config.video_preset.as_str())
        {
            config.video_preset = "medium".to_string();
        }
    }

    /// 确保配置文件存在，不存在则写入默认配置
    fn ensure_config_file(&self) {
        if !self.config_path.exists() {
            let _ = self.save();
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let config = self.config.lock().map_err(|e| e.to_string())?;
        let dir = Self::base_dir();
        fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
        let content = serde_json::to_string_pretty(&*config)
            .map_err(|e| format!("序列化配置失败: {e}"))?;
        fs::write(&self.config_path, content)
            .map_err(|e| format!("写入配置失败: {e}"))?;
        Ok(())
    }

    pub fn get_save_directory(&self) -> PathBuf {
        let config = self.config.lock().unwrap();
        match config.save_dir {
            SaveDir::Desktop => dirs::desktop_dir().unwrap_or_else(|| PathBuf::from(".")),
            SaveDir::Custom => config
                .custom_save_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| dirs::desktop_dir().unwrap_or_else(|| PathBuf::from("."))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 默认值测试 ==========

    #[test]
    fn should_have_correct_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.save_dir, SaveDir::Desktop);
        assert_eq!(config.custom_save_dir, None);
        assert_eq!(config.gif_fps, 10);
        assert_eq!(config.gif_max_duration_secs, 600);
        assert_eq!(config.video_fps, 15);
        assert_eq!(config.video_crf, 23);
        assert_eq!(config.video_preset, "medium");
        assert_eq!(config.video_max_duration_secs, 1800);
        assert!(!config.video_copy_to_clipboard);
        assert_eq!(config.version, CONFIG_FILE_VERSION);
    }

    #[test]
    fn should_have_platform_default_hotkeys() {
        let hotkeys = HotkeyConfig::default();
        if cfg!(target_os = "macos") {
            assert_eq!(hotkeys.snap, "Ctrl+Shift+1");
            assert_eq!(hotkeys.gif, "Ctrl+Shift+3");
            assert_eq!(hotkeys.video, "Ctrl+Shift+5");
        } else {
            assert_eq!(hotkeys.snap, "F1");
            assert_eq!(hotkeys.gif, "F3");
            assert_eq!(hotkeys.video, "F5");
        }
        assert_eq!(hotkeys.force_quit, "Ctrl+C,Ctrl+C");
    }

    // ========== 序列化/反序列化测试 ==========

    #[test]
    fn should_serialize_and_deserialize_config() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.gif_fps, 10);
        assert_eq!(restored.gif_max_duration_secs, 600);
        assert_eq!(restored.save_dir, SaveDir::Desktop);
        assert_eq!(restored.video_fps, 15);
        assert_eq!(restored.video_max_duration_secs, 1800);
        assert!(!restored.video_copy_to_clipboard);
        assert_eq!(restored.version, CONFIG_FILE_VERSION);
    }

    #[test]
    fn should_deserialize_with_missing_hotkeys_field() {
        // 旧版配置没有 hotkeys 字段，应该用默认值
        let json = r#"{"save_dir":"desktop","custom_save_dir":null,"gif_fps":10,"gif_max_duration_secs":180}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        // hotkeys 应该是默认值
        assert!(!config.hotkeys.snap.is_empty());
        assert!(!config.hotkeys.gif.is_empty());
    }

    #[test]
    fn should_deserialize_invalid_json_to_default() {
        let result: Result<AppConfig, _> = serde_json::from_str("not valid json");
        assert!(result.is_err());
        // load_from_file 内部会 unwrap_or_default
    }

    // ========== 范围校验测试 ==========

    #[test]
    fn should_clamp_gif_fps_too_high() {
        let json = r#"{"save_dir":"desktop","custom_save_dir":null,"gif_fps":1000,"gif_max_duration_secs":180}"#;
        let mut config: AppConfig = serde_json::from_str(json).unwrap();
        config.gif_fps = config.gif_fps.clamp(1, 60);
        assert_eq!(config.gif_fps, 60);
    }

    #[test]
    fn should_clamp_gif_fps_too_low() {
        let json = r#"{"save_dir":"desktop","custom_save_dir":null,"gif_fps":0,"gif_max_duration_secs":180}"#;
        let mut config: AppConfig = serde_json::from_str(json).unwrap();
        config.gif_fps = config.gif_fps.clamp(1, 60);
        assert_eq!(config.gif_fps, 1);
    }

    #[test]
    fn should_clamp_duration_too_high() {
        let json = r#"{"save_dir":"desktop","custom_save_dir":null,"gif_fps":10,"gif_max_duration_secs":99999}"#;
        let mut config: AppConfig = serde_json::from_str(json).unwrap();
        config.gif_max_duration_secs = config.gif_max_duration_secs.clamp(1, 600);
        assert_eq!(config.gif_max_duration_secs, 600);
    }

    // ========== SaveDir 枚举测试 ==========

    #[test]
    fn should_clamp_video_fps() {
        let json = r#"{"save_dir":"desktop","custom_save_dir":null,"gif_fps":10,"gif_max_duration_secs":180,"video_fps":999,"video_crf":23,"video_preset":"medium","video_max_duration_secs":300}"#;
        let mut config: AppConfig = serde_json::from_str(json).unwrap();
        config.video_fps = config.video_fps.clamp(5, 60);
        assert_eq!(config.video_fps, 60);
    }

    #[test]
    fn should_reset_invalid_video_preset() {
        let json = r#"{"save_dir":"desktop","custom_save_dir":null,"gif_fps":10,"gif_max_duration_secs":180,"video_fps":15,"video_crf":23,"video_preset":"invalid","video_max_duration_secs":300}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.video_preset, "invalid");
    }

    #[test]
    fn should_serialize_save_dir() {
        assert_eq!(serde_json::to_string(&SaveDir::Desktop).unwrap(), "\"desktop\"");
        assert_eq!(serde_json::to_string(&SaveDir::Custom).unwrap(), "\"custom\"");
    }
}
