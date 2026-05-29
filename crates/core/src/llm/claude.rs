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

        let resp = self
            .client
            .post(URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

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
