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
        parse_response(&v)
    }
}

/// Decode the structured JSON text from a Gemini `generateContent` envelope.
/// Split out from [`GeminiProvider::extract`] for unit-testing without a live
/// API call.
fn parse_response(v: &Value) -> Result<ExtractionResponse> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_candidate_part_text() {
        let v = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{ "text": "{\"is_subscription\":true,\"service_name\":\"YouTube Premium\",\"amount_minor\":1280,\"currency\":\"JPY\"}" }]
                }
            }]
        });
        let resp = parse_response(&v).expect("candidate text present");
        assert_eq!(resp.data["service_name"], "YouTube Premium");
        assert_eq!(resp.data["amount_minor"], 1280);
    }

    #[test]
    fn missing_candidate_is_error() {
        let v = serde_json::json!({ "candidates": [] });
        assert!(parse_response(&v).is_err());
    }

    #[test]
    fn non_json_text_is_error() {
        let v = serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "text": "not json" }] } }]
        });
        assert!(parse_response(&v).is_err());
    }
}
