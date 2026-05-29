use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ParsedSubscriptionHint;
use crate::llm::LlmClient;
use crate::{Error, Result};

/// A reference to a Gmail message we want to parse. The full body is fetched on
/// demand and never persisted — only `message_id` is stored to deduplicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessageRef {
    pub message_id: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub received_at: DateTime<Utc>,
}

/// Fetch a single Gmail message and run extraction.
///
/// TODO: implement Gmail API fetch + body decoding, then delegate to
/// [`super::extract_from_text`].
pub async fn parse_message(
    _provider: &LlmClient,
    _msg: &GmailMessageRef,
) -> Result<ParsedSubscriptionHint> {
    Err(Error::Parser("gmail::parse_message not yet implemented".into()))
}
