use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::config::{ConfigManager, SaveDir};
use crate::wal::logger::WalLogger;

pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let config_manager = app.state::<ConfigManager>();
    let config = config_manager.config.lock().unwrap().clone();

    let snap_item = MenuItem::with_id(app, "snap", "截屏\tF1", true, None::<&str>)?;
    let gif_item = MenuItem::with_id(app, "gif", "录制 GIF\tF3", true, None::<&str>)?;

    let sep1 = PredefinedMenuItem::separator(app)?;

    // 保存目录子菜单：显示当前自定义路径（如有）
    let dir_desktop = CheckMenuItem::with_id(
        app, "dir_desktop", "桌面", true,
        config.save_dir == SaveDir::Desktop, None::<&str>,
    )?;

    let custom_label = match &config.custom_save_dir {
        Some(path) => format!("自定义: {}", shorten_path(path)),
        None => "自定义...".to_string(),
    };
    let dir_custom = CheckMenuItem::with_id(
        app, "dir_custom", &custom_label, true,
        config.save_dir == SaveDir::Custom, None::<&str>,
    )?;

    let dir_change = MenuItem::with_id(app, "dir_change", "更改目录...", true, None::<&str>)?;
    let dir_submenu = Submenu::with_items(app, "保存目录", true, &[&dir_desktop, &dir_custom, &dir_change])?;

    let sep2 = PredefinedMenuItem::separator(app)?;

    let open_config = MenuItem::with_id(app, "open_config", "打开配置文件", true, None::<&str>)?;

    let sep3 = PredefinedMenuItem::separator(app)?;

    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &snap_item, &gif_item, &sep1,
        &dir_submenu, &sep2,
        &open_config, &sep3,
        &quit_item,
    ])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .build(app)?;

    Ok(())
}

/// 缩短路径显示：只保留最后两级目录
fn shorten_path(path: &str) -> String {
    let p = std::path::Path::new(path);
    let components: Vec<_> = p.components().collect();
    if components.len() <= 2 {
        return path.to_string();
    }
    let last_two: Vec<_> = components[components.len() - 2..].iter()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    format!(".../{}", last_two.join("/"))
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    let config_manager = app.state::<ConfigManager>();
    let wal = app.state::<WalLogger>();

    match id {
        "snap" => {
            wal.info("TRAY", "点击截屏菜单");
            let _ = crate::commands::snap_cmd::open_snap_overlay(app);
        }
        "gif" => {
            wal.info("TRAY", "点击录制GIF菜单");
            let _ = crate::commands::gif_cmd::open_gif_overlay(app);
        }
        "dir_desktop" => {
            let mut config = config_manager.config.lock().unwrap();
            config.save_dir = SaveDir::Desktop;
            drop(config);
            let _ = config_manager.save();
            wal.info("CONFIG", "配置变更 | key=save_dir | new=Desktop");
        }
        "dir_custom" => {
            // 已有自定义目录时，点击直接切换过去
            let has_custom = {
                let config = config_manager.config.lock().unwrap();
                config.custom_save_dir.is_some()
            };
            if has_custom {
                let mut config = config_manager.config.lock().unwrap();
                config.save_dir = SaveDir::Custom;
                drop(config);
                let _ = config_manager.save();
                wal.info("CONFIG", "配置变更 | key=save_dir | new=Custom");
            } else {
                // 没有自定义目录，弹出选择对话框
                pick_custom_dir(app);
            }
        }
        "dir_change" => {
            pick_custom_dir(app);
        }
        "open_config" => {
            // 用系统默认编辑器打开配置文件
            let config_path = ConfigManager::base_dir().join("config.json");
            wal.info("TRAY", &format!("打开配置文件: {}", config_path.to_string_lossy()));
            let _ = open::that(&config_path);
        }
        "quit" => {
            wal.info("APP", "应用退出");
            for label in &["overlay", "record-bar", "record-region"] {
                if let Some(win) = app.get_webview_window(label) {
                    let _ = win.destroy();
                }
            }
            crate::request_exit();
            app.exit(0);
        }
        _ => {}
    }
}

/// 弹出文件夹选择对话框，设置自定义保存目录
fn pick_custom_dir(app: &AppHandle) {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let config_manager = app_clone.state::<ConfigManager>();
            let wal = app_clone.state::<WalLogger>();
            let folder_str = folder.to_string_lossy().to_string();

            let mut config = config_manager.config.lock().unwrap();
            config.save_dir = SaveDir::Custom;
            config.custom_save_dir = Some(folder_str.clone());
            drop(config);
            let _ = config_manager.save();

            wal.info("CONFIG", &format!("配置变更 | key=custom_save_dir | new={folder_str}"));
        }
    });
}
