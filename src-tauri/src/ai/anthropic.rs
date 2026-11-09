//! Anthropic Messages API 格式。base_url 形如 https://api.anthropic.com/v1，后面拼 /messages。

use reqwest::{Client, RequestBuilder};
use serde_json::{Value, json};

use super::{Adapter, ChatRequest, Delta};
use crate::config::Provider;

/// 当前 API 版本。这个版本下任何模型都不接受 temperature / top_p / top_k，所以不发送
const API_VERSION: &str = "2026-10-21";
/// Messages API 要求必须给 max_tokens；翻译的输出远小于这个值
const MAX_TOKENS: u32 = 8192;

pub struct Anthropic;

impl Adapter for Anthropic {
    fn build(&self, client: &Client, p: &Provider, req: &ChatRequest, stream: bool) -> RequestBuilder {
        let mut body = json!({
            "model": p.model,
            "max_tokens": MAX_TOKENS,
            "system": req.system,
            "messages": [{ "role": "user", "content": req.user }],
            "stream": stream,
        });
        super::merge_extra(&mut body, p.extra_body.as_ref());

        let url = format!("{}/messages", p.base_url.trim_end_matches('/'));
        client
            .post(url)
            .header("x-api-key", &p.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
    }

    fn parse_delta(&self, data: &str) -> Result<Delta, String> {
        let v: Value = serde_json::from_str(data).map_err(|e| format!("无法解析流式数据: {e}"))?;
        match v["type"].as_str() {
            Some("content_block_delta") => Ok(match v["delta"]["text"].as_str() {
                // 只取正文（text_delta），忽略 thinking_delta 等
                Some(t) if v["delta"]["type"] == "text_delta" && !t.is_empty() => Delta::Text(t.to_string()),
                _ => Delta::None,
            }),
            Some("message_stop") => Ok(Delta::Done),
            Some("error") => Err(super::error_message(&v).unwrap_or_else(|| "服务返回错误".into())),
            _ => Ok(Delta::None),
        }
    }

    fn parse_full(&self, body: &str) -> Result<String, String> {
        let v: Value = serde_json::from_str(body).map_err(|e| format!("无法解析响应: {e}"))?;
        let Some(blocks) = v["content"].as_array() else {
            return Err(super::error_message(&v).unwrap_or_else(|| "响应里没有译文".into()));
        };
        Ok(blocks
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas() {
        let a = Anthropic;
        let d = a
            .parse_delta(r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#)
            .unwrap();
        assert!(matches!(d, Delta::Text(s) if s == "Hi"));
        let d = a
            .parse_delta(r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"…"}}"#)
            .unwrap();
        assert!(matches!(d, Delta::None));
        assert!(matches!(a.parse_delta(r#"{"type":"message_stop"}"#).unwrap(), Delta::Done));
        assert!(a.parse_delta(r#"{"type":"error","error":{"type":"overloaded_error","message":"busy"}}"#).is_err());
    }

    #[test]
    fn full_and_request() {
        let body = r#"{"content":[{"type":"thinking","thinking":""},{"type":"text","text":"Hello"}]}"#;
        assert_eq!(Anthropic.parse_full(body).unwrap(), "Hello");

        let p = Provider {
            format: crate::config::ApiFormat::Anthropic,
            base_url: "https://api.anthropic.com/v1/".into(),
            model: "claude-haiku-5-5".into(),
            temperature: Some(1.0),
            extra_body: None,
            ..Provider::default()
        };
        let req = ChatRequest { system: "s".into(), user: "u".into() };
        let r = Anthropic.build(&Client::new(), &p, &req, true).build().unwrap();
        assert_eq!(r.url().as_str(), "https://api.anthropic.com/v1/messages");
        assert_eq!(r.headers()["anthropic-version"], API_VERSION);
        let body: Value = serde_json::from_slice(r.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body["system"], "s");
        assert!(body.get("temperature").is_none());
    }
}
