mod ai;
mod config;
mod hotkeys;
mod lang;
mod orchestrator;
mod platform;
mod settings;
mod tray;
mod ui;

use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::MacosLauncher;
use ui::ShowMode;

pub fn run() {
    tauri::Builder::default()
        // 单实例必须最先注册：再次启动时只把已有窗口叫出来
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            ui::show_translate(app, ShowMode::Input, None);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(hotkeys::handle).build())
        // 开机自启写在注册表 Run 键里，状态以注册表为准，不进 config.toml
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        // 配置在创建窗口之前就要可用：前端页面可能在 setup 之前调用命令
        .manage(config::load())
        .manage(hotkeys::Hotkeys::default())
        .manage(orchestrator::Translator::new())
        .manage(orchestrator::PasteTarget::default())
        .manage(orchestrator::InlineState::default())
        .setup(|app| {
            let handle = app.handle();
            config::start_watching(handle)?;
            ui::setup(handle);
            tray::setup(handle)?;
            for warning in hotkeys::register_all(handle) {
                eprintln!("[relic] {warning}");
            }
            orchestrator::start_inline(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ui::hide,
            ui::get_theme,
            ui::resize_to_content,
            ui::fit_indicator,
            orchestrator::translate,
            orchestrator::cancel,
            orchestrator::copy_text,
            orchestrator::paste_back,
            settings::get_config,
            settings::set_config,
            settings::get_autostart,
            settings::set_autostart,
            settings::suspend_hotkeys,
            settings::list_languages,
            settings::open_config_file,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Relic 失败");
}

/// 配置变化（设置窗口、托盘改主题、手动改文件）后的统一处理。返回需要提示用户的问题
pub(crate) fn on_config_changed(app: &AppHandle, cfg: &config::Config) -> Vec<String> {
    ui::apply_theme(app);
    tray::sync_theme(app, cfg.general.theme);
    let warnings = hotkeys::register_all(app);
    orchestrator::start_inline(app);
    let _ = app.emit("relic://config-changed", ());
    warnings
}
