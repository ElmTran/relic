mod platform;
mod tray;
mod ui;

use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use ui::ShowMode;

pub fn run() {
    tauri::Builder::default()
        // 单实例必须最先注册：再次启动时只把已有窗口叫出来
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            ui::show_translate(app, ShowMode::Input, None);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        ui::show_translate(app, ShowMode::Input, Some(""));
                    }
                })
                .build(),
        )
        .manage(ui::UiState::default())
        .setup(|app| {
            let handle = app.handle();
            ui::setup(handle);
            tray::setup(handle)?;
            // 临时：输入翻译热键写死 Alt+W，M1 起从 config.toml 读取
            if let Err(e) = app.global_shortcut().register(Shortcut::new(Some(Modifiers::ALT), Code::KeyW)) {
                eprintln!("[relic] 注册热键 Alt+W 失败: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ui::hide, ui::get_theme])
        .run(tauri::generate_context!())
        .expect("启动 Relic 失败");
}
