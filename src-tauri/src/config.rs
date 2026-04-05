use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SaveDir {
    Desktop,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub save_dir: SaveDir,
    pub custom_save_dir: Option<String>,
    pub auto_start: bool,
    pub gif_fps: u16,
    pub gif_max_duration_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            save_dir: SaveDir::Desktop,
            custom_save_dir: None,
            auto_start: false,
            gif_fps: 10,
            gif_max_duration_secs: 180,
        }
    }
}

pub struct ConfigManager {
    pub config: Mutex<AppConfig>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = Self::config_file_path();
        let config = Self::load_from_file(&config_path);
        Self {
            config: Mutex::new(config),
            config_path,
        }
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
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
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
