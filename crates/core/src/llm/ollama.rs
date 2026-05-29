//! Ollama provider (local).
//!
//! Uses `/api/chat` with the `format` field set to the JSON Schema, which
//! Ollama 0.5+ honors as a structured-output constraint. Requires the model
//! itself to support JSON / tool output (most modern instruct models do).

use serde_json::Value;

use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

pub struct OllamaProvider {
    endpoint: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    #[must_use]
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            endpoint,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub async fn extract(&self, req: ExtractionRequest) -> Result<ExtractionResponse> {
        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": req.system_prompt},
                {"role": "user", "content": req.user_content},
            ],
            "format": req.schema,
            "stream": false,
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Llm(format!("ollama API {status}: {text}")));
        }

        let v: Value = resp.json().await?;
        let content = v
            .pointer("/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Llm("ollama: response missing message.content".into()))?;
        let data: Value = serde_json::from_str(content).map_err(|e| {
            Error::Llm(format!("ollama: response content is not valid JSON: {e}"))
        })?;

        Ok(ExtractionResponse {
            data,
            confidence: None,
        })
    }
}
