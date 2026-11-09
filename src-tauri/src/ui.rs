//! 窗口创建后的设置、定位、显示和隐藏，以及主题。

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Theme, WebviewWindow, WindowEvent,
};

use crate::config::{self, ThemeSetting};

pub const TRANSLATE: &str = "translate";
pub const INDICATOR: &str = "indicator";
pub const SETTINGS: &str = "settings";

/// 翻译窗口高度范围：最小值（逻辑像素）和占显示器工作区高度的最大比例
const MIN_HEIGHT: f64 = 100.0;
const MAX_HEIGHT_RATIO: f64 = 0.6;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShowMode {
    Selection,
    Input,
    /// 终端里双击 Alt：在 Relic 里打字，翻译后再按 Enter 粘贴回终端
    Compose,
}

#[derive(Clone, Serialize)]
struct ShowPayload<'a> {
    mode: ShowMode,
    /// None 表示保留窗口里现有内容
    text: Option<&'a str>,
    /// Compose 模式下固定的目标语言
    to: Option<&'a str>,
}

/// 浮标显示的状态
#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum IndicatorState {
    Working,
    Error { message: String },
}

fn translate_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(TRANSLATE)
}

fn indicator_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(INDICATOR)
}

pub fn setup(app: &AppHandle) {
    let win = translate_window(app).expect("translate 窗口未创建");
    crate::platform::effects::round_corners(&win);
    if let Some(indicator) = indicator_window(app) {
        crate::platform::effects::round_corners(&indicator);
        crate::platform::effects::make_no_activate(&indicator);
    }
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
    show(app, mode, text, None);
}

/// 终端里的内联翻译：弹出输入框，译成 to 后再按 Enter 粘贴回终端
pub fn show_compose(app: &AppHandle, to: &str) {
    show(app, ShowMode::Compose, Some(""), Some(to));
}

fn show(app: &AppHandle, mode: ShowMode, text: Option<&str>, to: Option<&str>) {
    let Some(win) = translate_window(app) else { return };
    let started = std::time::Instant::now();

    // 先推内容再显示，避免闪一下旧内容
    let _ = win.emit_to(TRANSLATE, "relic://show", ShowPayload { mode, text, to });
    if !win.is_visible().unwrap_or(false) {
        if let Some(pos) = placement(app, &win, mode) {
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

/// 翻译窗口的 HWND（用作剪贴板所有者，以及判断前台是不是自己）
#[cfg(windows)]
pub fn translate_hwnd(app: &AppHandle) -> Option<isize> {
    translate_window(app)?.hwnd().ok().map(|h| h.0 as isize)
}

/// 输入翻译：鼠标所在显示器工作区的中央。
/// 划词翻译、终端输入框：鼠标右下方，放不下时翻到上方/左侧，限制在工作区内。
fn placement(app: &AppHandle, win: &WebviewWindow, mode: ShowMode) -> Option<PhysicalPosition<i32>> {
    let cursor = app.cursor_position().ok()?;
    let monitor = app.monitor_from_point(cursor.x, cursor.y).ok().flatten()?;
    let area = monitor.work_area();
    let size = win.outer_size().ok()?;
    let (w, h) = (size.width as i32, size.height as i32);
    let (left, top) = (area.position.x, area.position.y);
    let (right, bottom) = (left + area.size.width as i32, top + area.size.height as i32);

    let (x, y) = match mode {
        ShowMode::Input => (left + (right - left - w) / 2, top + (bottom - top - h) / 2),
        ShowMode::Selection | ShowMode::Compose => {
            let gap = (12.0 * monitor.scale_factor()) as i32;
            let (cx, cy) = (cursor.x as i32, cursor.y as i32);
            let x = if cx + gap + w <= right { cx + gap } else { cx - gap - w };
            let y = if cy + gap + h <= bottom { cy + gap } else { cy - gap - h };
            (x.clamp(left, (right - w).max(left)), y.clamp(top, (bottom - h).max(top)))
        }
    };
    Some(PhysicalPosition::new(x, y))
}

/// 按配置设置窗口主题（原生 Acrylic 的色调跟着变），并通知前端
pub fn apply_theme(app: &AppHandle) {
    let theme = match config::get(app).general.theme {
        ThemeSetting::System => None,
        ThemeSetting::Light => Some(Theme::Light),
        ThemeSetting::Dark => Some(Theme::Dark),
    };
    for win in app.webview_windows().into_values() {
        let _ = win.set_theme(theme);
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
    let _ = app.emit("relic://theme", resolved_theme(app));
}

// ---- 设置窗口（按需创建，关闭即销毁）----

pub fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(SETTINGS) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let theme = match config::get(app).general.theme {
        ThemeSetting::System => None,
        ThemeSetting::Light => Some(Theme::Light),
        ThemeSetting::Dark => Some(Theme::Dark),
    };
    let url = tauri::WebviewUrl::App("src/windows/settings/index.html".into());
    let built = tauri::WebviewWindowBuilder::new(app, SETTINGS, url)
        .title("Relic 设置")
        .inner_size(780.0, 560.0)
        .min_inner_size(640.0, 440.0)
        .center()
        // 透明 + Mica：系统标题栏下面是 Win11 原生的 Mica 背景，打开时也不会白屏一闪
        .transparent(true)
        .effects(tauri::utils::config::WindowEffectsConfig {
            effects: vec![tauri::window::Effect::Mica],
            ..Default::default()
        })
        .theme(theme)
        .build();
    if let Err(e) = built {
        eprintln!("[relic] 打开设置窗口失败: {e}");
    }
}

// ---- 浮标 ----

/// 浮标的锚点：文本光标（屏幕物理像素）
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub x: i32,
    pub top: i32,
    pub bottom: i32,
}

/// 在文本光标下方显示浮标（放不下就放到上方）；拿不到光标就放在鼠标右下方。不抢焦点
pub fn show_indicator(app: &AppHandle, anchor: Option<Anchor>) {
    let Some(win) = indicator_window(app) else { return };
    let _ = win.emit_to(INDICATOR, "relic://indicator", IndicatorState::Working);
    let anchor = anchor.or_else(|| {
        let c = app.cursor_position().ok()?;
        let (x, y) = (c.x as i32 + 16, c.y as i32 + 16);
        Some(Anchor { x, top: y, bottom: y })
    });
    let Some(anchor) = anchor else { return };
    let Some(monitor) = app.monitor_from_point(anchor.x as f64, anchor.bottom as f64).ok().flatten() else { return };
    let area = monitor.work_area();
    let size = win.outer_size().unwrap_or_default();
    let (w, h) = (size.width as i32, size.height as i32);
    let gap = (6.0 * monitor.scale_factor()) as i32;
    let (left, top) = (area.position.x, area.position.y);
    let (right, bottom) = (left + area.size.width as i32, top + area.size.height as i32);

    let y = if anchor.bottom + gap + h <= bottom { anchor.bottom + gap } else { anchor.top - gap - h };
    let x = anchor.x.clamp(left, (right - w).max(left));
    crate::platform::effects::show_no_activate(&win, x, y.clamp(top, (bottom - h).max(top)));
}

pub fn indicator_error(app: &AppHandle, message: &str) {
    let _ = app.emit_to(INDICATOR, "relic://indicator", IndicatorState::Error { message: message.to_string() });
}

pub fn hide_indicator(app: &AppHandle) {
    if let Some(win) = indicator_window(app) {
        crate::platform::effects::hide_window(&win);
    }
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

/// 浮标按内容调整大小（逻辑像素）
#[tauri::command]
pub fn fit_indicator(window: WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    window.set_size(tauri::LogicalSize::new(width.ceil(), height.ceil())).map_err(|e| e.to_string())
}
