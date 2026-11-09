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
            // 前台是 Relic 自己的任何窗口（翻译、设置）都不算"其他应用"
            let from_other = fg.is_some_and(|h| !platform::focus::is_own(h));
            if from_other {
                *app.state::<PasteTarget>().0.lock().unwrap() = fg;
            }
            if let (ShowMode::Selection, true) = (mode, from_other) {
                let started = std::time::Instant::now();
                // 终端里 Ctrl+C 是中断，只用 UIA 读终端里的选区
                let allow_copy = !fg.is_some_and(platform::focus::is_terminal);
                let (text, via) = platform::selection::get(own.unwrap_or(0), allow_copy);
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
    use platform::focus;

    let target = app
        .state::<PasteTarget>()
        .0
        .lock()
        .unwrap()
        .ok_or("没有可回填的窗口，请先在其他应用里用热键呼出翻译窗口")?;
    let owner = ui::translate_hwnd(app).unwrap_or(0);
    let keep = config::get(app).general.keep_result_in_clipboard;

    let terminal = focus::is_terminal(target);
    paste(owner, text, keep, terminal, || {
        // 自己还是前台时切换，SetForegroundWindow 才会被允许
        focus::restore(target)?;
        ui::hide_translate(app);
        Ok(())
    })
}

/// 粘贴到当前焦点：快照剪贴板 → 写入文本 → before_paste（切窗口等）→ Ctrl+V → 等目标读完后恢复剪贴板。
/// keep=true 时译文作为正常内容留在剪贴板里，不恢复。terminal=true 时用 Shift+Insert 粘贴。
#[cfg(windows)]
fn paste(
    owner: isize,
    text: &str,
    keep: bool,
    terminal: bool,
    before_paste: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    use platform::{clipboard, input};

    let snapshot = if keep { None } else { clipboard::snapshot(owner).ok() };
    // 不保留时是临时内容，不进 Win+V 历史
    clipboard::set_text(owner, text, !keep)?;
    if let Err(e) = before_paste() {
        if let Some(s) = &snapshot {
            let _ = clipboard::restore(owner, s);
        }
        return Err(e);
    }
    // Ctrl+Enter 回填时 Ctrl 可能还按着
    input::wait_modifiers_released(std::time::Duration::from_secs(1));
    if terminal {
        input::shift_insert();
    } else {
        input::ctrl_v();
    }

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

// ---- M3：双击 Alt 内联翻译 ----

/// 内容超过这个字数多半是焦点不在输入框（Ctrl+A 选中了整个页面），不翻译
#[cfg(windows)]
const INLINE_MAX_CHARS: usize = 2000;

#[derive(Default)]
pub struct InlineState {
    busy: std::sync::atomic::AtomicBool,
    cancel: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

/// 内联翻译没有执行的原因：None 表示静默结束（无内容、已是目标语言、取消），Some 在浮标上显示
#[cfg(windows)]
type Skip = Option<String>;

/// 安装键盘钩子，开始监听双击 Alt。配置里关闭时不安装；之后打开会在配置变化时补装
pub fn start_inline(app: &AppHandle) {
    #[cfg(windows)]
    {
        use platform::keyhook::{self, HookEvent};
        static STARTED: std::sync::Once = std::sync::Once::new();

        let cfg = config::get(app);
        keyhook::set_interval(cfg.inline.interval_ms);
        if !cfg.inline.enabled {
            return;
        }
        STARTED.call_once(|| {
            let (tx, rx) = std::sync::mpsc::channel();
            keyhook::start(tx);
            let app = app.clone();
            std::thread::spawn(move || {
                for event in rx {
                    let state = app.state::<InlineState>();
                    match event {
                        HookEvent::DoubleAlt => {
                            // 同一时间只跑一个
                            if !state.busy.swap(true, std::sync::atomic::Ordering::AcqRel) {
                                let app = app.clone();
                                std::thread::spawn(move || {
                                    run_inline(&app);
                                    app.state::<InlineState>().busy.store(false, std::sync::atomic::Ordering::Release);
                                });
                            }
                        }
                        HookEvent::Escape => {
                            if let Some(tx) = state.cancel.lock().unwrap().take() {
                                let _ = tx.send(());
                            }
                        }
                    }
                }
            });
        });
    }
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(windows)]
fn run_inline(app: &AppHandle) {
    use platform::focus;

    let cfg = config::get(app);
    if !cfg.inline.enabled {
        return;
    }
    let Some(target) = focus::foreground() else { return };
    #[cfg(debug_assertions)]
    eprintln!(
        "[relic] 双击 Alt：前台 class={:?} process={:?} terminal={}",
        focus::class_name(target),
        focus::process_name(target),
        focus::is_terminal(target)
    );
    let owner = ui::translate_hwnd(app).unwrap_or(0);
    if focus::is_own(target) {
        return;
    }
    if let Some(name) = focus::process_name(target) {
        if cfg.inline.blacklist.iter().any(|b| b.eq_ignore_ascii_case(&name)) {
            return;
        }
    }
    // 终端的输入行读不到、也不能全选，改为弹出输入框，翻译后再按 Enter 粘贴回终端
    if focus::is_terminal(target) {
        *app.state::<PasteTarget>().0.lock().unwrap() = Some(target);
        ui::show_compose(app, &cfg.inline.target_lang);
        return;
    }

    let started = std::time::Instant::now();
    let anchor = focus::caret(target).map(|c| ui::Anchor { x: c.x, top: c.top, bottom: c.bottom });
    ui::show_indicator(app, anchor);
    let result = inline_translate(app, &cfg, target, owner);
    #[cfg(debug_assertions)]
    eprintln!("[relic] 内联翻译 {:?}：{:?}", started.elapsed(), result.as_ref().err());
    let _ = started;

    if let Err(Some(message)) = result {
        ui::indicator_error(app, &message);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    ui::hide_indicator(app);
}

#[cfg(windows)]
fn inline_translate(app: &AppHandle, cfg: &config::Config, target: isize, owner: isize) -> Result<(), Skip> {
    use platform::{focus, input, selection};

    // 有选中就只翻译选中部分，否则 Ctrl+A 选中整个输入框
    let mut text = selection::get(owner, true).0;
    let mut selected_all = false;
    if text.trim().is_empty() {
        input::wait_modifiers_released(std::time::Duration::from_millis(500));
        input::ctrl_a();
        std::thread::sleep(std::time::Duration::from_millis(30));
        text = selection::get(owner, true).0;
        selected_all = true;
    }
    // 不处理时收起自己选中的内容，免得用户接着打字把它覆盖掉
    let skip = |reason: Skip| {
        if selected_all {
            input::right();
        }
        Err(reason)
    };
    if text.trim().is_empty() {
        return skip(None);
    }
    if text.chars().count() > INLINE_MAX_CHARS {
        return skip(Some(format!("内容超过 {INLINE_MAX_CHARS} 字，已跳过")));
    }
    let to = cfg.inline.target_lang.as_str();
    let from = lang::detect(&text);
    if from == Some(to) {
        return skip(None);
    }

    let Some(provider) = cfg.ai.active_provider() else {
        return skip(Some(format!("找不到服务「{}」", cfg.ai.active)));
    };
    let req = ai::ChatRequest {
        system: ai::prompt::render(
            &cfg.prompts.inline,
            &from.map_or_else(|| "the detected language".into(), lang::prompt_name),
            &lang::prompt_name(to),
        ),
        user: text.clone(),
    };
    let client = app.state::<Translator>().client.clone();
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    *app.state::<InlineState>().cancel.lock().unwrap() = Some(cancel_tx);

    let outcome = tauri::async_runtime::block_on(async {
        let request = Box::pin(ai::complete(&client, provider, &req, |_| {}));
        match futures_util::future::select(request, cancel_rx).await {
            futures_util::future::Either::Left((result, _)) => Some(result),
            futures_util::future::Either::Right(_) => None,
        }
    });
    app.state::<InlineState>().cancel.lock().unwrap().take();

    let output = match outcome {
        None => return Err(None), // 用户按了 Esc
        Some(Err(message)) => return skip(Some(message)),
        Some(Ok(output)) => ai::prompt::clean(&output, &text),
    };
    if output.is_empty() {
        return skip(Some("译文为空".into()));
    }
    // 翻译期间用户切走了，就不要粘到别的窗口里
    if focus::foreground() != Some(target) {
        return Err(Some("窗口已切换，已取消".into()));
    }
    // 选区还在，粘贴即替换
    paste(owner, &output, false, false, || Ok(())).map_err(Some)
}
