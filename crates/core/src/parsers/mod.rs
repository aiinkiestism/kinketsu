//! Receipt parsing.
//!
//! The pipeline reads raw text from a source (an email body via [`gmail`], a
//! pasted snippet from the Scan UI, etc.), runs it through the configured
//! [`LlmClient`](crate::llm::LlmClient), and produces a
//! [`ParsedSubscriptionHint`]. Higher layers turn the hint into a
//! `DetectionEvent` for user review or persist it directly as a `Subscription`.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::llm::{ExtractionRequest, ExtractionResponse, LlmClient};
use crate::models::BillingCycle;
use crate::{Error, Result};

pub mod gmail;

/// Structured output of the extraction pipeline. Every field is optional because
/// real-world receipts vary widely in what they expose.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ParsedSubscriptionHint {
    pub service_name: Option<String>,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub billing_cycle: Option<BillingCycle>,
    pub payment_method_hint: Option<String>,
    pub charged_at: Option<NaiveDate>,
}

const SYSTEM_PROMPT: &str = "You are a structured-data extractor. Read the user-provided text (typically a subscription confirmation or renewal email) and extract the subscription fields. If a field is not clearly present, omit it. Use ISO 4217 currency codes (e.g. JPY, USD). For amount_minor, return the smallest unit of the currency — yen for JPY (no decimals), cents for USD/EUR/etc. billing_cycle must be one of: weekly, monthly, quarterly, semi_annual, annual, custom. charged_at is the ISO 8601 date the charge applies to (YYYY-MM-DD).";

fn extraction_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "service_name": {
                "type": "string",
                "description": "Human-readable name of the service (e.g. \"Netflix\")."
            },
            "amount_minor": {
                "type": "integer",
                "description": "Charge amount in minor units (yen for JPY; cents for USD)."
            },
            "currency": {
                "type": "string",
                "description": "ISO 4217 currency code."
            },
            "billing_cycle": {
                "type": "string",
                "enum": ["weekly", "monthly", "quarterly", "semi_annual", "annual", "custom"]
            },
            "payment_method_hint": {
                "type": "string",
                "description": "Free-text hint about the payment route (e.g. \"VISA ****1234\")."
            },
            "charged_at": {
                "type": "string",
                "format": "date",
                "description": "ISO 8601 date the charge applies to (YYYY-MM-DD)."
            }
        }
    })
}

/// Run the configured LLM provider against `content` and decode its structured
/// response as a [`ParsedSubscriptionHint`].
pub async fn extract_from_text(
    client: &LlmClient,
    content: String,
) -> Result<ParsedSubscriptionHint> {
    let req = ExtractionRequest {
        system_prompt: SYSTEM_PROMPT.into(),
        user_content: content,
        schema: extraction_schema(),
    };
    let resp: ExtractionResponse = client.extract(req).await?;
    serde_json::from_value(resp.data).map_err(Error::from)
}

const MANY_SYSTEM_PROMPT: &str = "You are reading bulk transaction data — typically a CSV export from a bank, card, or PayPal activity report; or multiple receipts concatenated together. Identify entries that look like recurring SUBSCRIPTION payments (same merchant + same amount + predictable cadence). Skip one-off purchases, transfers, refunds, and ATM withdrawals. For each subscription-like entry, extract one record using the same field conventions as for a single receipt: amount_minor in smallest units (yen for JPY, cents for USD), ISO 4217 currency, billing_cycle in {weekly, monthly, quarterly, semi_annual, annual, custom}, charged_at as YYYY-MM-DD. Return all detections as a JSON array under the key 'subscriptions'.";

fn many_extraction_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "subscriptions": {
                "type": "array",
                "items": extraction_schema()
            }
        },
        "required": ["subscriptions"]
    })
}

/// Bulk variant of [`extract_from_text`] — sends a corpus (CSV text or
/// concatenated receipts) and decodes the array of subscriptions the LLM
/// identifies. Returns an empty vector if the model finds none.
pub async fn extract_many_from_text(
    client: &LlmClient,
    content: String,
) -> Result<Vec<ParsedSubscriptionHint>> {
    let req = ExtractionRequest {
        system_prompt: MANY_SYSTEM_PROMPT.into(),
        user_content: content,
        schema: many_extraction_schema(),
    };
    let resp = client.extract(req).await?;
    let arr = resp
        .data
        .get("subscriptions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::Parser("LLM response missing 'subscriptions' array".into()))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        if let Ok(hint) = serde_json::from_value::<ParsedSubscriptionHint>(item.clone()) {
            out.push(hint);
        }
    }
    Ok(out)
}
