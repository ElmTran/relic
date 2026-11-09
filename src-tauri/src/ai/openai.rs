//! OpenAI Chat Completions 格式（DeepSeek 等兼容服务）。

use reqwest::{Client, RequestBuilder};
use serde_json::{Value, json};

use super::{Adapter, ChatRequest, Delta};
use crate::config::Provider;

pub struct OpenAi;

impl Adapter for OpenAi {
    fn build(&self, client: &Client, p: &Provider, req: &ChatRequest, stream: bool) -> RequestBuilder {
        let mut body = json!({
            "model": p.model,
            "messages": [
                { "role": "system", "content": req.system },
                { "role": "user", "content": req.user },
            ],
            "stream": stream,
        });
        if let Some(t) = p.temperature {
            body["temperature"] = json!(t);
        }
        super::merge_extra(&mut body, p.extra_body.as_ref());

        let url = format!("{}/chat/completions", p.base_url.trim_end_matches('/'));
        client.post(url).bearer_auth(&p.api_key).json(&body)
    }

    fn parse_delta(&self, data: &str) -> Result<Delta, String> {
        if data == "[DONE]" {
            return Ok(Delta::Done);
        }
        let v: Value = serde_json::from_str(data).map_err(|e| format!("无法解析流式数据: {e}"))?;
        if let Some(msg) = super::error_message(&v) {
            return Err(msg);
        }
        // 只取正文，忽略 reasoning_content
        Ok(match v["choices"][0]["delta"]["content"].as_str() {
            Some(s) if !s.is_empty() => Delta::Text(s.to_string()),
            _ => Delta::None,
        })
    }

    fn parse_full(&self, body: &str) -> Result<String, String> {
        let v: Value = serde_json::from_str(body).map_err(|e| format!("无法解析响应: {e}"))?;
        v["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| super::error_message(&v).unwrap_or_else(|| "响应里没有译文".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas() {
        let a = OpenAi;
        let d = a.parse_delta(r#"{"choices":[{"delta":{"content":"Hi"}}]}"#).unwrap();
        assert!(matches!(d, Delta::Text(s) if s == "Hi"));
        let d = a.parse_delta(r#"{"choices":[{"delta":{"reasoning_content":"..."}}]}"#).unwrap();
        assert!(matches!(d, Delta::None));
        assert!(matches!(a.parse_delta("[DONE]").unwrap(), Delta::Done));
        assert!(a.parse_delta(r#"{"error":{"message":"bad"}}"#).is_err());
    }

    #[test]
    fn body_has_extra() {
        let p = Provider::default();
        let req = ChatRequest { system: "s".into(), user: "u".into() };
        let r = OpenAi.build(&Client::new(), &p, &req, true).build().unwrap();
        assert_eq!(r.url().as_str(), "https://api.deepseek.com/chat/completions");
        let body: Value = serde_json::from_slice(r.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body["thinking"]["type"], "disabled");
        assert_eq!(body["messages"][1]["content"], "u");
    }
}
