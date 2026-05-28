use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::llm::LlmClient;
use crate::models::BillingCycle;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessageRef {
    pub message_id: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSubscriptionHint {
    pub service_name: Option<String>,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub billing_cycle: Option<BillingCycle>,
    pub payment_method_hint: Option<String>,
    pub charged_at: Option<NaiveDate>,
}

/// Parse a single Gmail message via the configured LLM provider.
///
/// TODO: fetch the message body through the Gmail API, run [`LlmClient::extract`]
/// with the receipt-extraction schema, return a [`ParsedSubscriptionHint`].
pub async fn parse_message(
    _provider: &LlmClient,
    _msg: &GmailMessageRef,
) -> Result<ParsedSubscriptionHint> {
    Err(Error::Parser("gmail::parse_message not yet implemented".into()))
}
