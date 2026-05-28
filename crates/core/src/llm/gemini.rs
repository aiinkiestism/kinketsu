use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

#[allow(dead_code)]
pub struct GeminiProvider {
    api_key: String,
    model: String,
}

impl GeminiProvider {
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub async fn extract(&self, _req: ExtractionRequest) -> Result<ExtractionResponse> {
        // TODO: POST https://generativelanguage.googleapis.com/v1beta/models/<model>:generateContent
        // with responseSchema for structured output.
        Err(Error::Llm("gemini: extract() not yet implemented".into()))
    }
}
