use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::ConfigManager;

pub enum Level {
    Info,
    Error,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Info => write!(f, "INFO"),
            Level::Error => write!(f, "ERROR"),
        }
    }
}

pub struct WalLogger {
    file: Mutex<std::fs::File>,
}

impl WalLogger {
    pub fn new() -> Self {
        let log_dir = Self::log_dir();
        fs::create_dir_all(&log_dir).expect("创建日志目录失败");

        let filename = Local::now().format("%Y%m%d%H%M%S").to_string();
        let log_path = log_dir.join(format!("{filename}.log"));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .expect("创建日志文件失败");

        let logger = Self {
            file: Mutex::new(file),
        };

        logger.cleanup_old_logs();
        logger
    }

    fn log_dir() -> PathBuf {
        ConfigManager::base_dir().join("logs")
    }

    pub fn log(&self, level: Level, tag: &str, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("[{timestamp}] [{level}] [{tag}] {message}\n");

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }

    pub fn info(&self, tag: &str, message: &str) {
        self.log(Level::Info, tag, message);
    }

    pub fn error(&self, tag: &str, message: &str) {
        self.log(Level::Error, tag, message);
    }

    fn cleanup_old_logs(&self) {
        let log_dir = Self::log_dir();
        let cutoff = Local::now() - chrono::Duration::days(30);
        let cutoff_str = cutoff.format("%Y%m%d%H%M%S").to_string();

        if let Ok(entries) = fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(stem) = name.strip_suffix(".log") {
                    if stem < cutoff_str.as_str() {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}
