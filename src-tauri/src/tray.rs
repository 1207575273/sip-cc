use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem},
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

    // 保存目录子菜单
    let dir_desktop = CheckMenuItem::with_id(
        app, "dir_desktop", "桌面（默认）", true,
        config.save_dir == SaveDir::Desktop, None::<&str>,
    )?;
    let dir_custom = MenuItem::with_id(app, "dir_custom", "自定义...", true, None::<&str>)?;
    let dir_submenu = Submenu::with_items(app, "保存目录", true, &[&dir_desktop, &dir_custom])?;

    let sep2 = PredefinedMenuItem::separator(app)?;

    let auto_start = CheckMenuItem::with_id(
        app, "auto_start", "开机自启", true, config.auto_start, None::<&str>,
    )?;

    let sep3 = PredefinedMenuItem::separator(app)?;

    let about_item = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &snap_item, &gif_item, &sep1,
        &dir_submenu, &sep2,
        &auto_start, &sep3,
        &about_item, &quit_item,
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
            // 任务 13 中实现：打开文件夹选择对话框
        }
        "auto_start" => {
            let mut config = config_manager.config.lock().unwrap();
            config.auto_start = !config.auto_start;
            let new_val = config.auto_start;
            drop(config);
            let _ = config_manager.save();
            wal.info("CONFIG", &format!("配置变更 | key=auto_start | new={new_val}"));
        }
        "about" => {
            // 任务 13 中实现
        }
        "quit" => {
            wal.info("APP", "应用退出");
            // 显式销毁所有窗口后退出
            for label in &["overlay", "record-bar", "record-region"] {
                if let Some(win) = app.get_webview_window(label) {
                    let _ = win.destroy();
                }
            }
            app.exit(0);
        }
        _ => {}
    }
}
