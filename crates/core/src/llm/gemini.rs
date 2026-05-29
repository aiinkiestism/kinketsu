//! Google Gemini provider.
//!
//! Uses `generateContent` with `generationConfig.responseSchema` to force
//! structured output. The schema is OpenAPI 3.0 subset — basic types and
//! enums work but advanced JSON Schema features may not.

use serde_json::Value;

use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

const URL_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub async fn extract(&self, req: ExtractionRequest) -> Result<ExtractionResponse> {
        let url = format!(
            "{URL_BASE}/{}:generateContent?key={}",
            self.model, self.api_key
        );
        let body = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": req.user_content}],
            }],
            "systemInstruction": {
                "parts": [{"text": req.system_prompt}],
            },
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": req.schema,
            },
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Llm(format!("gemini API {status}: {text}")));
        }

        let v: Value = resp.json().await?;
        let content = v
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Llm("gemini: response missing candidate text".into()))?;
        let data: Value = serde_json::from_str(content)
            .map_err(|e| Error::Llm(format!("gemini: response text is not valid JSON: {e}")))?;

        Ok(ExtractionResponse {
            data,
            confidence: None,
        })
    }
}
