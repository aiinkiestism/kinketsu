use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

#[allow(dead_code)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub async fn extract(&self, _req: ExtractionRequest) -> Result<ExtractionResponse> {
        // TODO: POST https://api.openai.com/v1/chat/completions with response_format=json_schema.
        Err(Error::Llm("openai: extract() not yet implemented".into()))
    }
}
