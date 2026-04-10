use std::io::Write;
use std::path::PathBuf;

use tauri::{AppHandle, Emitter};

use crate::config::ConfigManager;

const FFMPEG_VERSION: &str = "7.1";

pub fn get_ffmpeg_path() -> PathBuf {
    let bin_dir = ConfigManager::base_dir().join("bin");
    if cfg!(target_os = "windows") {
        bin_dir.join("ffmpeg.exe")
    } else {
        bin_dir.join("ffmpeg")
    }
}

pub fn is_ffmpeg_available() -> bool {
    let path = get_ffmpeg_path();
    if !path.exists() {
        return false;
    }
    std::process::Command::new(&path)
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn get_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "https://www.osxexperts.net/ffmpeg71arm.zip"
        } else {
            "https://evermeet.cx/ffmpeg/ffmpeg-7.1.zip"
        }
    } else {
        "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz"
    }
}

fn get_mirror_urls() -> Vec<&'static str> {
    if cfg!(target_os = "windows") {
        vec!["https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"]
    } else if cfg!(target_os = "macos") {
        vec![
            "https://evermeet.cx/ffmpeg/ffmpeg-7.1.zip",
            "https://www.osxexperts.net/ffmpeg71arm.zip",
        ]
    } else {
        vec!["https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz"]
    }
}

pub async fn download_ffmpeg(app: AppHandle) -> Result<PathBuf, String> {
    let bin_dir = ConfigManager::base_dir().join("bin");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("创建 bin 目录失败: {e}"))?;

    let urls: Vec<&str> = std::iter::once(get_download_url())
        .chain(get_mirror_urls())
        .collect();

    let mut last_err = String::new();

    for url in urls {
        match try_download(&app, url, &bin_dir).await {
            Ok(path) => {
                let version_file = bin_dir.join("ffmpeg.version");
                let _ = std::fs::write(&version_file, FFMPEG_VERSION);
                return Ok(path);
            }
            Err(e) => {
                last_err = e;
                let _ = app.emit("ffmpeg-download-error", &last_err);
            }
        }
    }

    Err(format!("所有下载源均失败: {last_err}"))
}

fn extension_from_url(url: &str) -> &str {
    if url.ends_with(".7z") {
        ".7z"
    } else if url.ends_with(".tar.xz") {
        ".tar.xz"
    } else {
        ".zip"
    }
}

async fn try_download(app: &AppHandle, url: &str, bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("HTTP 客户端初始化失败: {e}"))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let _ = app.emit("ffmpeg-download-start", total_size);

    let ext = extension_from_url(url);
    let tmp_path = bin_dir.join(format!("ffmpeg_download{ext}"));
    let mut file =
        std::fs::File::create(&tmp_path).map_err(|e| format!("创建临时文件失败: {e}"))?;

    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("写入失败: {e}"))?;
        downloaded += chunk.len() as u64;
        let _ = app.emit("ffmpeg-download-progress", (downloaded, total_size));
    }
    drop(file);

    extract_ffmpeg(&tmp_path, bin_dir)?;
    let _ = std::fs::remove_file(&tmp_path);

    #[cfg(not(target_os = "windows"))]
    {
        let ffmpeg_path = get_ffmpeg_path();
        let chmod = std::process::Command::new("chmod")
            .args(["+x", &ffmpeg_path.to_string_lossy().to_string()])
            .status();
        if let Err(e) = chmod {
            eprintln!("chmod +x ffmpeg 失败: {e}");
        }

        #[cfg(target_os = "macos")]
        {
            let xattr = std::process::Command::new("xattr")
                .args(["-cr", &ffmpeg_path.to_string_lossy().to_string()])
                .status();
            if let Err(e) = xattr {
                eprintln!("xattr -cr ffmpeg 失败: {e}");
            }
        }
    }

    Ok(get_ffmpeg_path())
}

fn extract_ffmpeg(archive_path: &PathBuf, bin_dir: &PathBuf) -> Result<(), String> {
    let path_str = archive_path.to_string_lossy();
    if path_str.ends_with(".7z") {
        extract_from_7z(archive_path, bin_dir)
    } else if path_str.ends_with(".tar.xz") {
        extract_from_tar_xz(archive_path, bin_dir)
    } else {
        extract_from_zip(archive_path, bin_dir)
    }
}

fn extract_from_7z(archive_path: &PathBuf, bin_dir: &PathBuf) -> Result<(), String> {
    let tmp_extract = bin_dir.join("_7z_tmp");
    let _ = std::fs::remove_dir_all(&tmp_extract);
    std::fs::create_dir_all(&tmp_extract)
        .map_err(|e| format!("创建解压临时目录失败: {e}"))?;

    sevenz_rust2::decompress_file(archive_path, &tmp_extract)
        .map_err(|e| format!("7z 解压失败: {e}"))?;

    let target = if cfg!(target_os = "windows") { "ffmpeg.exe" } else { "ffmpeg" };
    let found = find_file_recursive(&tmp_extract, target);

    match found {
        Some(src) => {
            let dest = bin_dir.join(target);
            std::fs::copy(&src, &dest)
                .map_err(|e| format!("复制 ffmpeg 失败: {e}"))?;
            let _ = std::fs::remove_dir_all(&tmp_extract);
            Ok(())
        }
        None => {
            let _ = std::fs::remove_dir_all(&tmp_extract);
            Err("7z 压缩包中未找到 ffmpeg 可执行文件".to_string())
        }
    }
}

fn find_file_recursive(dir: &PathBuf, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, name) {
                return Some(found);
            }
        } else if path.file_name().map(|n| n == name).unwrap_or(false) {
            return Some(path);
        }
    }
    None
}

fn extract_from_zip(zip_path: &PathBuf, bin_dir: &PathBuf) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("打开压缩包失败: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析 ZIP 失败: {e}"))?;

    let target_name = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读取条目失败: {e}"))?;
        let name = entry.name().to_string();

        if name.ends_with(target_name)
            && !name.contains("ffprobe")
            && !name.contains("ffplay")
        {
            let dest = bin_dir.join(target_name);
            let mut out =
                std::fs::File::create(&dest).map_err(|e| format!("创建文件失败: {e}"))?;
            std::io::copy(&mut entry, &mut out).map_err(|e| format!("解压失败: {e}"))?;
            return Ok(());
        }
    }

    Err("压缩包中未找到 ffmpeg 可执行文件".to_string())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn extract_from_tar_xz(tar_path: &PathBuf, bin_dir: &PathBuf) -> Result<(), String> {
    let tmp_extract = bin_dir.join("_tar_tmp");
    let _ = std::fs::remove_dir_all(&tmp_extract);
    std::fs::create_dir_all(&tmp_extract)
        .map_err(|e| format!("创建解压临时目录失败: {e}"))?;

    let status = std::process::Command::new("tar")
        .args([
            "xf",
            &tar_path.to_string_lossy(),
            "-C",
            &tmp_extract.to_string_lossy(),
        ])
        .status()
        .map_err(|e| format!("tar 执行失败: {e}"))?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp_extract);
        return Err("tar 解压失败".to_string());
    }

    let found = find_file_recursive(&tmp_extract, "ffmpeg");
    match found {
        Some(src) => {
            let dest = bin_dir.join("ffmpeg");
            std::fs::copy(&src, &dest)
                .map_err(|e| format!("复制 ffmpeg 失败: {e}"))?;
            let _ = std::fs::remove_dir_all(&tmp_extract);
            Ok(())
        }
        None => {
            let _ = std::fs::remove_dir_all(&tmp_extract);
            Err("tar 包中未找到 ffmpeg 可执行文件".to_string())
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn extract_from_tar_xz(_: &PathBuf, _: &PathBuf) -> Result<(), String> {
    Err("当前平台不支持 tar.xz 解压".to_string())
}
