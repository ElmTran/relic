mod ai;
mod config;
mod lang;
mod orchestrator;
mod platform;
mod tray;
mod ui;

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use ui::ShowMode;

/// 当前注册的输入翻译热键
#[derive(Default)]
struct Hotkeys {
    input: Mutex<Option<Shortcut>>,
}

pub fn run() {
    tauri::Builder::default()
        // 单实例必须最先注册：再次启动时只把已有窗口叫出来
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            ui::show_translate(app, ShowMode::Input, None);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let is_input = app.state::<Hotkeys>().input.lock().unwrap().as_ref() == Some(shortcut);
                    if is_input {
                        ui::show_translate(app, ShowMode::Input, Some(""));
                    }
                })
                .build(),
        )
        .manage(Hotkeys::default())
        .manage(orchestrator::Translator::new())
        .setup(|app| {
            let handle = app.handle();
            config::init(handle)?;
            ui::setup(handle);
            tray::setup(handle)?;
            register_hotkeys(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ui::hide,
            ui::get_theme,
            ui::resize_to_content,
            orchestrator::translate,
            orchestrator::cancel,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Relic 失败");
}

/// 按配置注册热键；没变就不动
fn register_hotkeys(app: &AppHandle) {
    let wanted = config::get(app).hotkeys.input.clone();
    let parsed = match wanted.parse::<Shortcut>() {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("[relic] 热键「{wanted}」格式不对: {e}");
            None
        }
    };
    let state = app.state::<Hotkeys>();
    let mut current = state.input.lock().unwrap();
    if *current == parsed {
        return;
    }
    let gs = app.global_shortcut();
    if let Some(old) = current.take() {
        let _ = gs.unregister(old);
    }
    if let Some(s) = parsed {
        match gs.register(s) {
            Ok(()) => *current = Some(s),
            Err(e) => eprintln!("[relic] 注册热键「{wanted}」失败（可能被其他程序占用）: {e}"),
        }
    }
}

/// 配置变化（托盘改主题或手动改文件）后的统一处理
pub(crate) fn on_config_changed(app: &AppHandle, cfg: &config::Config) {
    ui::apply_theme(app);
    tray::sync_theme(app, cfg.general.theme);
    register_hotkeys(app);
    let _ = app.emit("relic://config-changed", ());
}
