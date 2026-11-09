//! 托盘：显示窗口、主题、打开设置、退出。

use tauri::{
    AppHandle, Manager, Wry,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::config::{self, ThemeSetting};
use crate::ui::{self, ShowMode};

const THEMES: [(ThemeSetting, &str, &str); 3] = [
    (ThemeSetting::System, "theme-system", "跟随系统"),
    (ThemeSetting::Light, "theme-light", "浅色"),
    (ThemeSetting::Dark, "theme-dark", "深色"),
];

/// 主题菜单项，配置变化时同步勾选状态
struct ThemeItems(Vec<(ThemeSetting, CheckMenuItem<Wry>)>);

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let current = config::get(app).general.theme;
    let theme_items = THEMES
        .iter()
        .map(|&(s, id, text)| Ok((s, CheckMenuItem::with_id(app, id, text, true, s == current, None::<&str>)?)))
        .collect::<tauri::Result<Vec<_>>>()?;

    let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let theme = Submenu::with_items(app, "主题", true, &[&theme_items[0].1, &theme_items[1].1, &theme_items[2].1])?;
    let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &theme, &settings, &PredefinedMenuItem::separator(app)?, &quit])?;
    app.manage(ThemeItems(theme_items));

    TrayIconBuilder::with_id("main")
        .icon(ui::small_icon())
        .tooltip("Relic")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => ui::show_translate(app, ShowMode::Input, None),
            "settings" => ui::open_settings(app),
            "quit" => app.exit(0),
            id => {
                if let Some(&(setting, _, _)) = THEMES.iter().find(|t| t.1 == id) {
                    config::set_theme(app, setting);
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

pub fn sync_theme(app: &AppHandle, theme: ThemeSetting) {
    if let Some(items) = app.try_state::<ThemeItems>() {
        for (s, item) in &items.0 {
            let _ = item.set_checked(*s == theme);
        }
    }
}
