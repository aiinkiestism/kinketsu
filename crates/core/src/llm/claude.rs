use super::{ExtractionRequest, ExtractionResponse};
use crate::{Error, Result};

#[allow(dead_code)]
pub struct ClaudeProvider {
    api_key: String,
    model: String,
}

impl ClaudeProvider {
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub async fn extract(&self, _req: ExtractionRequest) -> Result<ExtractionResponse> {
        // TODO: POST https://api.anthropic.com/v1/messages with tool-use forcing the JSON schema.
        Err(Error::Llm("claude: extract() not yet implemented".into()))
    }
}
