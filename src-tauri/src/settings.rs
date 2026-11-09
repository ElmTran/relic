//! 设置窗口用到的命令：读写配置、开机自启、录入热键时暂停全局热键、语言列表。

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::Shortcut;
use ts_rs::TS;

use crate::config::{self, Config};
use crate::{hotkeys, lang};

#[derive(Serialize, TS)]
#[ts(export)]
pub struct Language {
    pub code: String,
    pub name: String,
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> Config {
    (*config::get(&app)).clone()
}

/// 校验后保存。返回需要提示的问题（如热键被占用），校验不通过直接报错
#[tauri::command]
pub fn set_config(app: AppHandle, config: Config) -> Result<Vec<String>, String> {
    validate(&config)?;
    config::save(&app, config)
}

fn validate(c: &Config) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for p in &c.ai.providers {
        if p.id.trim().is_empty() {
            return Err("服务名称不能为空".into());
        }
        if !ids.insert(p.id.as_str()) {
            return Err(format!("服务名称「{}」重复", p.id));
        }
        if let Some(t) = p.temperature {
            if !(0.0..=2.0).contains(&t) {
                return Err(format!("「{}」的 temperature 要在 0 到 2 之间", p.id));
            }
        }
        if p.extra_body.as_ref().is_some_and(|v| !v.is_object()) {
            return Err(format!("「{}」的额外参数必须是 JSON 对象", p.id));
        }
    }
    if !c.ai.providers.is_empty() && !ids.contains(c.ai.active.as_str()) {
        return Err("当前使用的服务不存在".into());
    }
    let (sel, input) = (c.hotkeys.selection.trim(), c.hotkeys.input.trim());
    for (name, key) in [("划词翻译", sel), ("输入翻译", input)] {
        if !key.is_empty() && key.parse::<Shortcut>().is_err() {
            return Err(format!("{name}热键「{key}」格式不对"));
        }
    }
    if !sel.is_empty() && sel.eq_ignore_ascii_case(input) {
        return Err("划词翻译和输入翻译不能用同一个热键".into());
    }
    if !(100..=1000).contains(&c.inline.interval_ms) {
        return Err("双击间隔要在 100 到 1000 毫秒之间".into());
    }
    Ok(())
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled { manager.enable() } else { manager.disable() }.map_err(|e| format!("设置开机自启失败: {e}"))
}

/// 录入热键期间暂停全局热键；恢复时返回注册失败的提示
#[tauri::command]
pub fn suspend_hotkeys(app: AppHandle, suspended: bool) -> Vec<String> {
    hotkeys::set_suspended(&app, suspended)
}

#[tauri::command]
pub fn list_languages() -> Vec<Language> {
    lang::all().map(|(code, name)| Language { code: code.into(), name: name.into() }).collect()
}

/// 高级项（如 extra_body）直接在编辑器里改
#[tauri::command]
pub fn open_config_file(app: AppHandle) {
    crate::platform::shell::open_path(&config::path(&app));
}
