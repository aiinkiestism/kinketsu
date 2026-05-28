use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

#[allow(dead_code)]
pub struct LmStudioProvider {
    endpoint: String,
    model: String,
}

impl LmStudioProvider {
    #[must_use]
    pub fn new(endpoint: String, model: String) -> Self {
        Self { endpoint, model }
    }

    pub async fn extract(&self, _req: ExtractionRequest) -> Result<ExtractionResponse> {
        // TODO: POST {endpoint}/v1/chat/completions (OpenAI-compatible) with response_format=json_schema.
        Err(Error::Llm("lmstudio: extract() not yet implemented".into()))
    }
}
