//! 全局热键：按配置注册、配置变化时重新注册、设置窗口录入热键时暂停。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::ui::ShowMode;
use crate::{config, orchestrator};

/// 当前注册的热键
#[derive(Default)]
pub struct Hotkeys {
    selection: Mutex<Option<Shortcut>>,
    input: Mutex<Option<Shortcut>>,
    /// 设置窗口正在录入热键：全部注销，否则按下要录的组合键会直接触发
    suspended: AtomicBool,
}

pub fn handle(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let hotkeys = app.state::<Hotkeys>();
    if hotkeys.selection.lock().unwrap().as_ref() == Some(shortcut) {
        orchestrator::on_hotkey(app, ShowMode::Selection);
    } else if hotkeys.input.lock().unwrap().as_ref() == Some(shortcut) {
        orchestrator::on_hotkey(app, ShowMode::Input);
    }
}

/// 按配置注册热键；没变的不动。返回需要提示用户的问题
pub fn register_all(app: &AppHandle) -> Vec<String> {
    let hotkeys = app.state::<Hotkeys>();
    if hotkeys.suspended.load(Ordering::Acquire) {
        return vec![];
    }
    let cfg = config::get(app);
    [
        register(app, &hotkeys.selection, "划词翻译", &cfg.hotkeys.selection),
        register(app, &hotkeys.input, "输入翻译", &cfg.hotkeys.input),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn register(app: &AppHandle, slot: &Mutex<Option<Shortcut>>, name: &str, wanted: &str) -> Option<String> {
    let parsed = match wanted.parse::<Shortcut>() {
        Ok(s) => Some(s),
        Err(_) if wanted.trim().is_empty() => None,
        Err(_) => return Some(format!("{name}热键「{wanted}」格式不对")),
    };
    let mut current = slot.lock().unwrap();
    if *current == parsed {
        return None;
    }
    let gs = app.global_shortcut();
    if let Some(old) = current.take() {
        let _ = gs.unregister(old);
    }
    let s = parsed?;
    match gs.register(s) {
        Ok(()) => {
            *current = Some(s);
            None
        }
        Err(_) => Some(format!("{name}热键「{wanted}」注册失败，可能被其他程序占用")),
    }
}

pub fn set_suspended(app: &AppHandle, suspended: bool) -> Vec<String> {
    let hotkeys = app.state::<Hotkeys>();
    hotkeys.suspended.store(suspended, Ordering::Release);
    if !suspended {
        return register_all(app);
    }
    let gs = app.global_shortcut();
    for slot in [&hotkeys.selection, &hotkeys.input] {
        if let Some(s) = slot.lock().unwrap().take() {
            let _ = gs.unregister(s);
        }
    }
    vec![]
}
