//! 各功能的完整流程。M1：输入翻译（检测语言 → 定方向 → 流式请求 → 后处理）。

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, async_runtime::JoinHandle, ipc::Channel};
use ts_rs::TS;

use crate::{ai, config, lang};

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
