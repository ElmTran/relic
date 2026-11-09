//! AI 服务：适配器只负责构造请求和解析响应，发送、SSE、错误处理是公共逻辑。
//! 取消由调用方 abort 任务实现。

mod anthropic;
mod gemini;
mod openai;
pub mod prompt;
mod sse;

use std::time::Duration;

use futures_util::StreamExt;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::Value;

use crate::config::{ApiFormat, Provider};

pub struct ChatRequest {
    pub system: String,
    pub user: String,
}

pub enum Delta {
    Text(String),
    None,
    Done,
}

trait Adapter: Sync {
    fn build(&self, client: &Client, p: &Provider, req: &ChatRequest, stream: bool) -> RequestBuilder;
    fn parse_delta(&self, data: &str) -> Result<Delta, String>;
    fn parse_full(&self, body: &str) -> Result<String, String>;
}

fn adapter(format: ApiFormat) -> Result<&'static dyn Adapter, String> {
    match format {
        ApiFormat::Openai => Ok(&openai::OpenAi),
        ApiFormat::Anthropic => Ok(&anthropic::Anthropic),
        ApiFormat::Gemini => Ok(&gemini::Gemini),
    }
}

/// 共用的 HTTP 客户端：复用连接，降低延迟。默认读取系统代理。
pub fn client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        // 不带 User-Agent 会被部分服务的防火墙直接 403
        .user_agent(concat!("Relic/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("创建 HTTP 客户端失败")
}

/// 发送请求。流式时每收到一段正文就回调 on_delta，最后返回完整原始输出。
pub async fn complete(
    client: &Client,
    provider: &Provider,
    req: &ChatRequest,
    mut on_delta: impl FnMut(&str),
) -> Result<String, String> {
    if provider.api_key.trim().is_empty() {
        return Err("还没有填写 API key：托盘菜单 → 打开设置".into());
    }
    let adapter = adapter(provider.format)?;
    let resp = adapter
        .build(client, provider, req, provider.stream)
        .send()
        .await
        .map_err(describe_reqwest_error)?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(describe_status(status, &body));
    }

    if !provider.stream {
        let body = resp.text().await.map_err(describe_reqwest_error)?;
        return adapter.parse_full(&body);
    }

    let mut parser = sse::SseParser::default();
    let mut events = Vec::new();
    let mut full = String::new();
    let mut stream = resp.bytes_stream();
    let mut done = false;
    while !done {
        match stream.next().await {
            Some(chunk) => parser.push(&chunk.map_err(describe_reqwest_error)?, &mut events),
            None => {
                parser.finish(&mut events);
                done = true;
            }
        }
        for data in events.drain(..) {
            match adapter.parse_delta(&data)? {
                Delta::Text(t) => {
                    full.push_str(&t);
                    on_delta(&t);
                }
                Delta::None => {}
                Delta::Done => done = true,
            }
        }
    }
    Ok(full)
}

/// 把 extra_body 深度合并进请求体：对象逐层合并，其他值直接覆盖
fn merge_extra(body: &mut Value, extra: Option<&Value>) {
    if let Some(extra) = extra {
        deep_merge(body, extra);
    }
}

fn deep_merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (k, v) in patch {
                deep_merge(base.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (base, patch) => *base = patch.clone(),
    }
}

/// 各家通用的 {"error": {"message": ...}} 或 {"error": "..."}
fn error_message(v: &Value) -> Option<String> {
    let e = v.get("error")?;
    e.get("message").and_then(Value::as_str).or_else(|| e.as_str()).map(str::to_string)
}

fn describe_status(status: StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body).ok().and_then(|v| error_message(&v));
    let hint = match status.as_u16() {
        401 => "API key 无效",
        402 => "账户余额不足",
        403 => "没有权限",
        404 => "地址或模型不存在",
        429 => "请求太频繁或额度用完",
        500..=599 => "服务端出错",
        _ => "请求失败",
    };
    match detail {
        Some(d) => format!("{hint}（{}）：{d}", status.as_u16()),
        None => format!("{hint}（{}）", status.as_u16()),
    }
}

fn describe_reqwest_error(e: reqwest::Error) -> String {
    if e.is_timeout() {
        "连接超时".into()
    } else if e.is_connect() {
        "无法连接到服务，检查网络或代理".into()
    } else {
        format!("网络错误：{e}")
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::config::Config;

    /// 用本机 %APPDATA%/com.relic.app/config.toml 里的服务实际请求一次。
    /// 手动运行：cargo test live_ -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_translate() {
        let path = std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.relic.app/config.toml");
        let cfg: Config = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let provider = cfg.ai.active_provider().unwrap();
        let client = client();
        for (prompt, text, from, to) in [
            (&cfg.prompts.translate, "今天天气不错，我们去公园散步吧。", "Simplified Chinese", "English"),
            (&cfg.prompts.translate, "The quick brown fox jumps over the lazy dog.", "English", "Simplified Chinese"),
            (&cfg.prompts.translate, "鉴于目前项目进度有所延迟，我们需要重新评估资源分配并调整后续的里程碑安排。", "Simplified Chinese", "English"),
            (&cfg.prompts.inline, "不好意思，我刚才在开会没看到消息，这个问题我晚点再跟你确认一下。", "Simplified Chinese", "English"),
            (&cfg.prompts.inline, "这个方案我觉得总体上没什么问题，不过成本方面可能还需要再优化一下。", "Simplified Chinese", "English"),
        ] {
            let req = ChatRequest { system: prompt::render(prompt, from, to), user: text.into() };
            let start = std::time::Instant::now();
            let mut first = None;
            let mut chunks = 0;
            let out = tauri::async_runtime::block_on(complete(&client, provider, &req, |_| {
                first.get_or_insert(start.elapsed());
                chunks += 1;
            }))
            .unwrap();
            println!(
                "{text}\n  → {}\n  首字 {:?}，总计 {:?}，{chunks} 段",
                prompt::clean(&out, text),
                first.unwrap_or_default(),
                start.elapsed()
            );
        }
    }
}
