//! Anthropic Claude provider.
//!
//! Forces structured output via tool-use: we declare a `record_subscription`
//! tool whose `input_schema` is the caller's JSON Schema, set
//! `tool_choice = {type: tool, name: ...}`, and read the `tool_use.input`
//! block back as the response data.

use serde_json::Value;

use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

const URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const TOOL_NAME: &str = "record_subscription";
const MAX_TOKENS: u32 = 1024;
const MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_AFTER_SECS: u64 = 30;
const MAX_RETRY_AFTER_SECS: u64 = 120;

pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub async fn extract(&self, req: ExtractionRequest) -> Result<ExtractionResponse> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": req.system_prompt,
            "tools": [{
                "name": TOOL_NAME,
                "description": "Record the structured subscription extracted from the user content.",
                "input_schema": req.schema,
            }],
            "tool_choice": {"type": "tool", "name": TOOL_NAME},
            "messages": [
                {"role": "user", "content": req.user_content},
            ],
        });

        let mut attempt: u32 = 0;
        let resp = loop {
            attempt += 1;
            let r = self
                .client
                .post(URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?;

            let status = r.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < MAX_RETRIES {
                // Respect Retry-After when present (Anthropic also exposes
                // anthropic-ratelimit-* but Retry-After is the canonical hint).
                let wait = r
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or_else(|| DEFAULT_RETRY_AFTER_SECS * u64::from(attempt))
                    .min(MAX_RETRY_AFTER_SECS);
                let body_text = r.text().await.unwrap_or_default();
                tracing::warn!(
                    "claude 429 (attempt {attempt}/{MAX_RETRIES}), sleeping {wait}s: {body_text}"
                );
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                continue;
            }

            break r;
        };

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Llm(format!("claude API {status}: {text}")));
        }

        let v: Value = resp.json().await?;
        let input = v
            .get("content")
            .and_then(Value::as_array)
            .and_then(|arr| {
                arr.iter()
                    .find(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
                    .and_then(|b| b.get("input"))
                    .cloned()
            })
            .ok_or_else(|| Error::Llm("claude: response missing tool_use input".into()))?;

        Ok(ExtractionResponse {
            data: input,
            confidence: None,
        })
    }
}
