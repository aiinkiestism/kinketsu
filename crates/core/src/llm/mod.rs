//! LLM provider abstraction.
//!
//! Providers are selected per-user via [`LlmConfig`] and dispatched through the
//! [`LlmClient`] enum (enum dispatch instead of trait objects — keeps the
//! provider set explicit and avoids `dyn` + `async-trait` overhead).

use serde::{Deserialize, Serialize};

use crate::Result;

pub mod claude;
pub mod gemini;
pub mod lmstudio;
pub mod ollama;
pub mod openai;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum LlmConfig {
    Claude {
        api_key: String,
        model: String,
    },
    #[serde(rename = "openai")]
    OpenAi {
        api_key: String,
        model: String,
    },
    Gemini {
        api_key: String,
        model: String,
    },
    Ollama {
        endpoint: String,
        model: String,
    },
    #[serde(rename = "lmstudio")]
    LmStudio {
        endpoint: String,
        model: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRequest {
    pub system_prompt: String,
    pub user_content: String,
    /// JSON Schema describing the structured output the provider must produce.
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResponse {
    pub data: serde_json::Value,
    pub confidence: Option<f32>,
}

pub enum LlmClient {
    Claude(claude::ClaudeProvider),
    OpenAi(openai::OpenAiProvider),
    Gemini(gemini::GeminiProvider),
    Ollama(ollama::OllamaProvider),
    LmStudio(lmstudio::LmStudioProvider),
}

impl LlmClient {
    #[must_use]
    pub fn from_config(config: LlmConfig) -> Self {
        match config {
            LlmConfig::Claude { api_key, model } => {
                Self::Claude(claude::ClaudeProvider::new(api_key, model))
            }
            LlmConfig::OpenAi { api_key, model } => {
                Self::OpenAi(openai::OpenAiProvider::new(api_key, model))
            }
            LlmConfig::Gemini { api_key, model } => {
                Self::Gemini(gemini::GeminiProvider::new(api_key, model))
            }
            LlmConfig::Ollama { endpoint, model } => {
                Self::Ollama(ollama::OllamaProvider::new(endpoint, model))
            }
            LlmConfig::LmStudio { endpoint, model } => {
                Self::LmStudio(lmstudio::LmStudioProvider::new(endpoint, model))
            }
        }
    }

    pub async fn extract(&self, req: ExtractionRequest) -> Result<ExtractionResponse> {
        match self {
            Self::Claude(c) => c.extract(req).await,
            Self::OpenAi(c) => c.extract(req).await,
            Self::Gemini(c) => c.extract(req).await,
            Self::Ollama(c) => c.extract(req).await,
            Self::LmStudio(c) => c.extract(req).await,
        }
    }

    #[must_use]
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Claude(_) => "claude",
            Self::OpenAi(_) => "openai",
            Self::Gemini(_) => "gemini",
            Self::Ollama(_) => "ollama",
            Self::LmStudio(_) => "lmstudio",
        }
    }
}
