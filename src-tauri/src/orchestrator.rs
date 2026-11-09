//! 各功能的完整流程：输入翻译（检测语言 → 定方向 → 流式请求 → 后处理）、划词、复制、回填。

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, async_runtime::JoinHandle, ipc::Channel};
use ts_rs::TS;

use crate::ui::{self, ShowMode};
use crate::{ai, config, lang, platform};

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct TranslateRequest {
    pub text: String,
    /// 手动指定目标语言；null 表示自动方向
    pub to: Option<String>,
}

#[derive(Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum TranslateEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        from_name: String,
        to: String,
        to_name: String,
        /// 点击方向标签时切换到的目标语言
        swap_to: String,
    },
    Delta { text: String },
    /// 后处理后的完整译文，前端用它替换流式拼出的内容
    Done { text: String },
    Error { message: String },
}

pub struct Translator {
    client: reqwest::Client,
    current: Mutex<Option<JoinHandle<()>>>,
}

impl Translator {
    pub fn new() -> Self {
        Self { client: ai::client(), current: Mutex::new(None) }
    }

    fn replace_task(&self, task: Option<JoinHandle<()>>) {
        let old = std::mem::replace(&mut *self.current.lock().unwrap(), task);
        if let Some(old) = old {
            old.abort();
        }
    }
}

#[tauri::command]
pub fn translate(app: AppHandle, req: TranslateRequest, channel: Channel<TranslateEvent>) {
    let text = req.text.trim().to_string();
    if text.is_empty() {
        return;
    }
    let cfg = config::get(&app);
    let general = &cfg.general;
    let from = lang::detect(&text);
    let to = req
        .to
        .unwrap_or_else(|| lang::auto_target(from, &general.primary_lang, &general.secondary_lang).to_string());
    let swap_to = if to == general.primary_lang { &general.secondary_lang } else { &general.primary_lang };

    let _ = channel.send(TranslateEvent::Started {
        from_name: from.map_or_else(|| "自动".into(), lang::display_name),
        to: to.clone(),
        to_name: lang::display_name(&to),
        swap_to: swap_to.clone(),
    });

    let translator = app.state::<Translator>();
    let client = translator.client.clone();
    let task = tauri::async_runtime::spawn(async move {
        let Some(provider) = cfg.ai.active_provider() else {
            let _ = channel.send(TranslateEvent::Error {
                message: format!("找不到 ai.active 指定的服务「{}」", cfg.ai.active),
            });
            return;
        };
        let from_name = from.map_or_else(|| "the detected language".into(), lang::prompt_name);
        let req = ai::ChatRequest {
            system: ai::prompt::render(&cfg.prompts.translate, &from_name, &lang::prompt_name(&to)),
            user: text.clone(),
        };
        let result = ai::complete(&client, provider, &req, |delta| {
            let _ = channel.send(TranslateEvent::Delta { text: delta.to_string() });
        })
        .await;
        let _ = channel.send(match result {
            Ok(out) => TranslateEvent::Done { text: ai::prompt::clean(&out, &text) },
            Err(message) => TranslateEvent::Error { message },
        });
    });
    // 新请求中止还在进行的旧请求
    translator.replace_task(Some(task));
}

#[tauri::command]
pub fn cancel(app: AppHandle) {
    app.state::<Translator>().replace_task(None);
}

// ---- M2：划词、复制、回填 ----

/// 回填的目标窗口：最近一次用热键呼出翻译窗口时的前台窗口
#[derive(Default)]
pub struct PasteTarget(Mutex<Option<isize>>);

/// 热键触发。取词有等待，放到独立线程，不卡住事件循环
pub fn on_hotkey(app: &AppHandle, mode: ShowMode) {
    #[cfg(windows)]
    platform::input::mask_alt_if_down();

    let app = app.clone();
    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            let own = ui::translate_hwnd(&app);
            let fg = platform::focus::foreground();
            let from_other = fg.is_some() && fg != own;
            if from_other {
                *app.state::<PasteTarget>().0.lock().unwrap() = fg;
            }
            if let (ShowMode::Selection, true) = (mode, from_other) {
                let started = std::time::Instant::now();
                let (text, via) = platform::selection::get(own.unwrap_or(0));
                #[cfg(debug_assertions)]
                eprintln!("[relic] 取词 {via:?}：{} 字，{:?}", text.chars().count(), started.elapsed());
                let _ = (started, via);
                // 没选中任何文字时按输入翻译处理
                if text.trim().is_empty() {
                    ui::show_translate(&app, ShowMode::Input, Some(""));
                } else {
                    ui::show_translate(&app, ShowMode::Selection, Some(&text));
                }
                return;
            }
            // 在 Relic 自己的窗口里按划词热键：保留现有内容
            if !from_other && matches!(mode, ShowMode::Selection) {
                ui::show_translate(&app, ShowMode::Selection, None);
                return;
            }
        }
        ui::show_translate(&app, ShowMode::Input, Some(""));
    });
}

/// 复制译文：正常写入剪贴板（会进 Win+V 历史，这是用户主动复制）
#[tauri::command]
pub fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    #[cfg(windows)]
    return platform::clipboard::set_text(ui::translate_hwnd(&app).unwrap_or(0), &text, false);
    #[cfg(not(windows))]
    {
        let _ = (app, text);
        Err("当前平台不支持".into())
    }
}

/// 回填：译文写入剪贴板 → 切回原窗口 → 隐藏翻译窗口 → Ctrl+V → 恢复剪贴板
#[tauri::command]
pub async fn paste_back(app: AppHandle, text: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || paste_back_blocking(&app, &text))
        .await
        .map_err(|e| e.to_string())?
}

/// 恢复剪贴板前等目标程序读完，太早恢复会粘贴出旧内容
#[cfg(windows)]
const PASTE_SETTLE: std::time::Duration = std::time::Duration::from_millis(300);

#[cfg(windows)]
fn paste_back_blocking(app: &AppHandle, text: &str) -> Result<(), String> {
    use platform::{clipboard, focus, input};

    let target = app
        .state::<PasteTarget>()
        .0
        .lock()
        .unwrap()
        .ok_or("没有可回填的窗口，请先在其他应用里用热键呼出翻译窗口")?;
    let owner = ui::translate_hwnd(app).unwrap_or(0);
    let keep = config::get(app).general.keep_result_in_clipboard;

    let snapshot = if keep { None } else { clipboard::snapshot(owner).ok() };
    // 不保留时是临时内容，不进 Win+V 历史
    clipboard::set_text(owner, text, !keep)?;

    // 自己还是前台时切换，SetForegroundWindow 才会被允许
    if let Err(e) = focus::restore(target) {
        if let Some(s) = &snapshot {
            let _ = clipboard::restore(owner, s);
        }
        return Err(e);
    }
    ui::hide_translate(app);
    // Ctrl+Enter 回填时 Ctrl 可能还按着
    input::wait_modifiers_released(std::time::Duration::from_secs(1));
    input::ctrl_v();

    if let Some(s) = snapshot {
        std::thread::sleep(PASTE_SETTLE);
        clipboard::restore(owner, &s)?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn paste_back_blocking(_app: &AppHandle, _text: &str) -> Result<(), String> {
    Err("当前平台不支持".into())
}
