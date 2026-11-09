//! 托盘：显示窗口、切换主题、退出。设置入口在 M4 加。

use tauri::{
    AppHandle,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::ui::{self, ShowMode, ThemeSetting};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let theme_items = [
        (ThemeSetting::System, CheckMenuItem::with_id(app, "theme-system", "跟随系统", true, true, None::<&str>)?),
        (ThemeSetting::Light, CheckMenuItem::with_id(app, "theme-light", "浅色", true, false, None::<&str>)?),
        (ThemeSetting::Dark, CheckMenuItem::with_id(app, "theme-dark", "深色", true, false, None::<&str>)?),
    ];
    let theme = Submenu::with_items(
        app,
        "主题",
        true,
        &[&theme_items[0].1, &theme_items[1].1, &theme_items[2].1],
    )?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &theme, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("缺少应用图标").clone())
        .tooltip("Relic")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => ui::show_translate(app, ShowMode::Input, None),
            "quit" => app.exit(0),
            id => {
                if let Some((setting, _)) = theme_items.iter().find(|(_, item)| item.id() == id) {
                    ui::set_theme(app, *setting);
                    for (s, item) in &theme_items {
                        let _ = item.set_checked(s == setting);
                    }
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                ui::show_translate(tray.app_handle(), ShowMode::Input, None);
            }
        })
        .build(app)?;
    Ok(())
}
