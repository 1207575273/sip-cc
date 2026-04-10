use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::commands::snap_cmd::macos_overlay_window_count;
use crate::config::{ConfigManager, SaveDir};
use crate::wal::logger::WalLogger;

/// 构建托盘菜单（从当前配置读取快捷键和保存目录）
fn build_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let config_manager = app.state::<ConfigManager>();
    let config = config_manager.config.lock().unwrap().clone();

    let snap_key = &config.hotkeys.snap;
    let gif_key = &config.hotkeys.gif;
    let snap_item = MenuItem::with_id(app, "snap", &format!("截屏\t{snap_key}"), true, None::<&str>)?;
    let gif_item = MenuItem::with_id(app, "gif", &format!("录制 GIF\t{gif_key}"), true, None::<&str>)?;

    let video_key = &config.hotkeys.video;
    let video_item = MenuItem::with_id(app, "video", &format!("录制视频\t{video_key}"), true, None::<&str>)?;

    let sep1 = PredefinedMenuItem::separator(app)?;

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

    let hotkey_settings = MenuItem::with_id(app, "hotkey_settings", "快捷键设置", true, None::<&str>)?;
    let open_config = MenuItem::with_id(app, "open_config", "打开配置文件", true, None::<&str>)?;

    let sep3 = PredefinedMenuItem::separator(app)?;

    let about_item = MenuItem::with_id(app, "about", "关于 sip-cc", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &snap_item, &gif_item, &video_item, &sep1,
        &dir_submenu, &sep2,
        &hotkey_settings, &open_config, &sep3,
        &about_item,
        &quit_item,
    ])?;

    Ok(menu)
}

/// 首次创建托盘图标 + 菜单
pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app)?;

    let mut builder = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(move |app, event| {
            handle_menu_event(app, event.id.as_ref());
        });

    if cfg!(target_os = "macos") {
        builder = builder.show_menu_on_left_click(true);
    } else {
        builder = builder
            .show_menu_on_left_click(false)
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    let app = tray.app_handle();
                    let _ = crate::commands::snap_cmd::open_snap_overlay(app);
                }
            });
    }

    builder.build(app)?;

    Ok(())
}

/// 刷新托盘菜单（快捷键/保存目录变更后调用，更新菜单文字）
pub fn refresh_tray_menu(app: &AppHandle) {
    if let Ok(menu) = build_menu(app) {
        // 获取已有的 tray icon 并替换菜单
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_menu(Some(menu));
        }
    }
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
        "video" => {
            wal.info("TRAY", "点击录制视频菜单");
            let _ = crate::commands::video_cmd::open_video_overlay(app);
        }
        "dir_desktop" => {
            let mut config = config_manager.config.lock().unwrap();
            config.save_dir = SaveDir::Desktop;
            drop(config);
            let _ = config_manager.save();
            wal.info("CONFIG", "配置变更 | key=save_dir | new=Desktop");
            refresh_tray_menu(app);
        }
        "dir_custom" => {
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
                refresh_tray_menu(app);
            } else {
                pick_custom_dir(app);
            }
        }
        "dir_change" => {
            pick_custom_dir(app);
        }
        "hotkey_settings" => {
            open_hotkey_settings(app);
        }
        "open_config" => {
            let config_path = ConfigManager::base_dir().join("config.json");
            wal.info("TRAY", &format!("打开配置文件: {}", config_path.to_string_lossy()));
            let _ = open::that(&config_path);
        }
        "about" => {
            show_about(app);
        }
        "quit" => {
            wal.info("APP", "应用退出");
            if let Some(win) = app.get_webview_window("overlay") {
                let _ = win.destroy();
            }
            for i in 0..macos_overlay_window_count(app) {
                let label = format!("overlay-{}", i);
                if let Some(win) = app.get_webview_window(&label) {
                    let _ = win.destroy();
                }
            }
            for label in &["record-bar", "record-region", "record-info", "ffmpeg-download"] {
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

fn open_hotkey_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("hotkey-settings") {
        let _ = win.set_focus();
        return;
    }

    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let _ = WebviewWindowBuilder::new(
        app, "hotkey-settings",
        WebviewUrl::App("index.html?view=hotkey-settings".into()),
    )
    .title("快捷键设置")
    .inner_size(400.0, 340.0)
    .resizable(false)
    .center()
    .build();
}

fn show_about(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("about") {
        let _ = win.set_focus();
        return;
    }

    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let _ = WebviewWindowBuilder::new(
        app, "about",
        WebviewUrl::App("index.html?view=about".into()),
    )
    .title("关于 sip-cc")
    .inner_size(300.0, 360.0)
    .resizable(false)
    .center()
    .build();
}

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
            refresh_tray_menu(&app_clone);
        }
    });
}
