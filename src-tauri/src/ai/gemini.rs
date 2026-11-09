//! Google Gemini generateContent 格式。base_url 形如 https://generativelanguage.googleapis.com/v1beta，
//! 后面拼 /models/{model}:generateContent（流式用 :streamGenerateContent?alt=sse）。

use reqwest::{Client, RequestBuilder};
use serde_json::{Value, json};

use super::{Adapter, ChatRequest, Delta};
use crate::config::Provider;

pub struct Gemini;

/// 一个响应块里所有正文片段（跳过思考片段 thought=true）
fn text_of(v: &Value) -> String {
    v["candidates"][0]["content"]["parts"]
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter(|p| p["thought"] != true)
                .filter_map(|p| p["text"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

impl Adapter for Gemini {
    fn build(&self, client: &Client, p: &Provider, req: &ChatRequest, stream: bool) -> RequestBuilder {
        let mut body = json!({
            "systemInstruction": { "parts": [{ "text": req.system }] },
            "contents": [{ "role": "user", "parts": [{ "text": req.user }] }],
        });
        if let Some(t) = p.temperature {
            body["generationConfig"] = json!({ "temperature": t });
        }
        super::merge_extra(&mut body, p.extra_body.as_ref());

        let base = p.base_url.trim_end_matches('/');
        let url = if stream {
            format!("{base}/models/{}:streamGenerateContent?alt=sse", p.model)
        } else {
            format!("{base}/models/{}:generateContent", p.model)
        };
        client.post(url).header("x-goog-api-key", &p.api_key).json(&body)
    }

    /// Gemini 的流没有结束标记，连接关闭即结束
    fn parse_delta(&self, data: &str) -> Result<Delta, String> {
        let v: Value = serde_json::from_str(data).map_err(|e| format!("无法解析流式数据: {e}"))?;
        if let Some(msg) = super::error_message(&v) {
            return Err(msg);
        }
        let text = text_of(&v);
        Ok(if text.is_empty() { Delta::None } else { Delta::Text(text) })
    }

    fn parse_full(&self, body: &str) -> Result<String, String> {
        let v: Value = serde_json::from_str(body).map_err(|e| format!("无法解析响应: {e}"))?;
        if let Some(msg) = super::error_message(&v) {
            return Err(msg);
        }
        let text = text_of(&v);
        if text.is_empty() {
            // 被安全策略拦下时没有正文，只有 finishReason
            let reason = v["candidates"][0]["finishReason"].as_str().unwrap_or("未知原因");
            return Err(format!("响应里没有译文（{reason}）"));
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas() {
        let g = Gemini;
        let d = g
            .parse_delta(r#"{"candidates":[{"content":{"parts":[{"text":"Hel"},{"text":"lo"}],"role":"model"}}]}"#)
            .unwrap();
        assert!(matches!(d, Delta::Text(s) if s == "Hello"));
        let d = g.parse_delta(r#"{"candidates":[{"content":{"parts":[{"text":"…","thought":true}]}}]}"#).unwrap();
        assert!(matches!(d, Delta::None));
        assert!(g.parse_delta(r#"{"error":{"code":400,"message":"bad"}}"#).is_err());
    }

    #[test]
    fn request_and_deep_merge() {
        let p = Provider {
            format: crate::config::ApiFormat::Gemini,
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            model: "gemini-flash-latest".into(),
            temperature: Some(0.7),
            // 深度合并：只加 thinkingConfig，不覆盖 temperature
            extra_body: Some(json!({ "generationConfig": { "thinkingConfig": { "thinkingBudget": 0 } } })),
            ..Provider::default()
        };
        let req = ChatRequest { system: "s".into(), user: "u".into() };
        let r = Gemini.build(&Client::new(), &p, &req, true).build().unwrap();
        assert_eq!(
            r.url().as_str(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:streamGenerateContent?alt=sse"
        );
        let body: Value = serde_json::from_slice(r.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body["generationConfig"]["thinkingConfig"]["thinkingBudget"], 0);
        assert!((body["generationConfig"]["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);
    }
}
