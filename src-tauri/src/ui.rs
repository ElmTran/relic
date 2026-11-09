//! 窗口创建后的设置、定位、显示和隐藏，以及主题。

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Theme, WebviewWindow, WindowEvent};

pub const TRANSLATE: &str = "translate";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Default)]
pub struct UiState {
    theme: Mutex<ThemeSetting>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShowMode {
    #[allow(dead_code)] // M2 划词时使用
    Selection,
    Input,
}

#[derive(Clone, Serialize)]
struct ShowPayload<'a> {
    mode: ShowMode,
    /// None 表示保留窗口里现有内容
    text: Option<&'a str>,
}

fn translate_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(TRANSLATE)
}

pub fn setup(app: &AppHandle) {
    let win = translate_window(app).expect("translate 窗口未创建");
    crate::platform::effects::round_corners(&win);

    let handle = app.clone();
    win.on_window_event(move |event| match event {
        // Alt+F4 等关闭请求只隐藏，窗口常驻
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide_translate(&handle);
        }
        WindowEvent::ThemeChanged(_) => emit_theme(&handle),
        _ => {}
    });
}

/// 显示翻译窗口。已经显示时只替换内容，不移动位置。
pub fn show_translate(app: &AppHandle, mode: ShowMode, text: Option<&str>) {
    let Some(win) = translate_window(app) else { return };
    let started = std::time::Instant::now();

    // 先推内容再显示，避免闪一下旧内容
    let _ = win.emit_to(TRANSLATE, "relic://show", ShowPayload { mode, text });
    if !win.is_visible().unwrap_or(false) {
        if let Some(pos) = center_on_cursor_monitor(app, &win) {
            let _ = win.set_position(pos);
        }
        let _ = win.show();
    }
    let _ = win.set_focus();

    #[cfg(debug_assertions)]
    eprintln!("[relic] show_translate: {:?}", started.elapsed());
    let _ = started;
}

pub fn hide_translate(app: &AppHandle) {
    if let Some(win) = translate_window(app) {
        let _ = win.hide();
    }
}

/// 鼠标所在显示器工作区的中央
fn center_on_cursor_monitor(app: &AppHandle, win: &WebviewWindow) -> Option<PhysicalPosition<i32>> {
    let cursor = app.cursor_position().ok()?;
    let monitor = app.monitor_from_point(cursor.x, cursor.y).ok().flatten()?;
    let area = monitor.work_area();
    let size = win.outer_size().ok()?;
    Some(PhysicalPosition::new(
        area.position.x + (area.size.width as i32 - size.width as i32) / 2,
        area.position.y + (area.size.height as i32 - size.height as i32) / 2,
    ))
}

pub fn set_theme(app: &AppHandle, setting: ThemeSetting) {
    *app.state::<UiState>().theme.lock().unwrap() = setting;
    if let Some(win) = translate_window(app) {
        // 原生 Acrylic 的色调跟着窗口主题变
        let _ = win.set_theme(match setting {
            ThemeSetting::System => None,
            ThemeSetting::Light => Some(Theme::Light),
            ThemeSetting::Dark => Some(Theme::Dark),
        });
    }
    emit_theme(app);
}

fn resolved_theme(app: &AppHandle) -> &'static str {
    let setting = *app.state::<UiState>().theme.lock().unwrap();
    let theme = match setting {
        ThemeSetting::Light => Theme::Light,
        ThemeSetting::Dark => Theme::Dark,
        ThemeSetting::System => translate_window(app)
            .and_then(|w| w.theme().ok())
            .unwrap_or(Theme::Light),
    };
    match theme {
        Theme::Dark => "dark",
        _ => "light",
    }
}

fn emit_theme(app: &AppHandle) {
    let _ = app.emit_to(TRANSLATE, "relic://theme", resolved_theme(app));
}

// ---- 命令 ----

#[tauri::command]
pub fn hide(app: AppHandle) {
    hide_translate(&app);
}

#[tauri::command]
pub fn get_theme(app: AppHandle) -> &'static str {
    resolved_theme(&app)
}
