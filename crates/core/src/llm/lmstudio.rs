//! LM Studio provider (local).
//!
//! LM Studio exposes an OpenAI-compatible REST API on the local endpoint.
//! We reuse the OpenAI Chat Completions shape with `response_format` set to
//! `json_schema`.

use serde_json::Value;

use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

pub struct LmStudioProvider {
    endpoint: String,
    model: String,
    client: reqwest::Client,
}

impl LmStudioProvider {
    #[must_use]
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            endpoint,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub async fn extract(&self, req: ExtractionRequest) -> Result<ExtractionResponse> {
        let url = format!(
            "{}/v1/chat/completions",
            self.endpoint.trim_end_matches('/')
        );
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": req.system_prompt},
                {"role": "user", "content": req.user_content},
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "subscription_extraction",
                    "schema": req.schema,
                },
            },
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Llm(format!("lmstudio API {status}: {text}")));
        }

        let v: Value = resp.json().await?;
        parse_response(&v)
    }
}

/// Decode the JSON-schema content string from LM Studio's OpenAI-compatible
/// envelope. Split out from [`LmStudioProvider::extract`] for unit-testing
/// without a live server.
fn parse_response(v: &Value) -> Result<ExtractionResponse> {
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Llm("lmstudio: response missing message.content".into()))?;
    let data: Value = serde_json::from_str(content)
        .map_err(|e| Error::Llm(format!("lmstudio: response content is not valid JSON: {e}")))?;

    Ok(ExtractionResponse {
        data,
        confidence: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_compatible_content() {
        let v = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\"is_subscription\":true,\"service_name\":\"Adobe\",\"amount_minor\":2480,\"currency\":\"JPY\"}"
                }
            }]
        });
        let resp = parse_response(&v).expect("content present");
        assert_eq!(resp.data["service_name"], "Adobe");
        assert_eq!(resp.data["amount_minor"], 2480);
    }

    #[test]
    fn missing_content_is_error() {
        let v = serde_json::json!({ "choices": [] });
        assert!(parse_response(&v).is_err());
    }

    #[test]
    fn non_json_content_is_error() {
        let v = serde_json::json!({
            "choices": [{ "message": { "content": "Sure! Here is the data." } }]
        });
        assert!(parse_response(&v).is_err());
    }
}
