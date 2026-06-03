//! OpenAI provider.
//!
//! Uses the Chat Completions endpoint with `response_format = json_schema`
//! so the model is required to emit a JSON object matching the caller's
//! schema.

use serde_json::Value;

use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

const URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
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

        let resp = self
            .client
            .post(URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Llm(format!("openai API {status}: {text}")));
        }

        let v: Value = resp.json().await?;
        parse_response(&v)
    }
}

/// Decode the JSON-schema content string from a Chat Completions envelope.
/// Split out from [`OpenAiProvider::extract`] so it can be unit-tested without
/// a live API call.
fn parse_response(v: &Value) -> Result<ExtractionResponse> {
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Llm("openai: response missing message.content".into()))?;
    let data: Value = serde_json::from_str(content)
        .map_err(|e| Error::Llm(format!("openai: response content is not valid JSON: {e}")))?;

    Ok(ExtractionResponse {
        data,
        confidence: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_schema_content_string() {
        let v = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\"is_subscription\":true,\"service_name\":\"Spotify\",\"amount_minor\":980,\"currency\":\"JPY\"}"
                }
            }]
        });
        let resp = parse_response(&v).expect("content present");
        assert_eq!(resp.data["service_name"], "Spotify");
        assert_eq!(resp.data["amount_minor"], 980);
    }

    #[test]
    fn missing_content_is_error() {
        let v = serde_json::json!({ "choices": [{ "message": { "role": "assistant" } }] });
        assert!(parse_response(&v).is_err());
    }

    #[test]
    fn non_json_content_is_error() {
        let v = serde_json::json!({
            "choices": [{ "message": { "content": "I'm sorry, I can't help with that." } }]
        });
        assert!(parse_response(&v).is_err());
    }
}
