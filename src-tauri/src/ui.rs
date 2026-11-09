//! 窗口创建后的设置、定位、显示和隐藏，以及主题。

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Theme, WebviewWindow, WindowEvent,
};

use crate::config::{self, ThemeSetting};

pub const TRANSLATE: &str = "translate";

/// 翻译窗口高度范围：最小值（逻辑像素）和占显示器工作区高度的最大比例
const MIN_HEIGHT: f64 = 100.0;
const MAX_HEIGHT_RATIO: f64 = 0.6;

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
    apply_theme(app);

    let handle = app.clone();
    win.on_window_event(move |event| match event {
        // Alt+F4 等关闭请求只隐藏，窗口常驻
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide_translate(&handle);
        }
        WindowEvent::Focused(false) if config::get(&handle).general.hide_on_blur => hide_translate(&handle),
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

/// 按配置设置窗口主题（原生 Acrylic 的色调跟着变），并通知前端
pub fn apply_theme(app: &AppHandle) {
    if let Some(win) = translate_window(app) {
        let _ = win.set_theme(match config::get(app).general.theme {
            ThemeSetting::System => None,
            ThemeSetting::Light => Some(Theme::Light),
            ThemeSetting::Dark => Some(Theme::Dark),
        });
    }
    emit_theme(app);
}

fn resolved_theme(app: &AppHandle) -> &'static str {
    let theme = match config::get(app).general.theme {
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

/// 高度随内容变化（宽度不动），超过上限由前端滚动；窗口底部不超出工作区
#[tauri::command]
pub fn resize_to_content(window: WebviewWindow, height: f64) -> Result<(), String> {
    let run = || -> tauri::Result<()> {
        let Some(monitor) = window.current_monitor()? else { return Ok(()) };
        let scale = window.scale_factor()?;
        let area = monitor.work_area();
        let max = (area.size.height as f64 * MAX_HEIGHT_RATIO).max(MIN_HEIGHT * scale);
        let target = (height * scale).round().clamp(MIN_HEIGHT * scale, max) as u32;

        let size = window.inner_size()?;
        if size.height == target {
            return Ok(());
        }
        window.set_size(PhysicalSize::new(size.width, target))?;

        let pos = window.outer_position()?;
        let area_bottom = area.position.y + area.size.height as i32;
        if pos.y + target as i32 > area_bottom {
            let y = (area_bottom - target as i32).max(area.position.y);
            window.set_position(PhysicalPosition::new(pos.x, y))?;
        }
        Ok(())
    };
    run().map_err(|e| e.to_string())
}
