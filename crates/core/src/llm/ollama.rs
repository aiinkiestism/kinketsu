use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

#[allow(dead_code)]
pub struct OllamaProvider {
    endpoint: String,
    model: String,
}

impl OllamaProvider {
    #[must_use]
    pub fn new(endpoint: String, model: String) -> Self {
        Self { endpoint, model }
    }

    pub async fn extract(&self, _req: ExtractionRequest) -> Result<ExtractionResponse> {
        // TODO: POST {endpoint}/api/chat with `format` set to the JSON schema for structured output.
        Err(Error::Llm("ollama: extract() not yet implemented".into()))
    }
}
