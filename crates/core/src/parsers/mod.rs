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

const SYSTEM_PROMPT: &str = "You are evaluating an email to decide whether it represents a real recurring subscription, then extracting structured fields only if it is.\n\nFirst set `is_subscription`:\n- TRUE only when the email is a direct billing or renewal notification from the merchant itself for a recurring service (Netflix renewal, Spotify monthly charge, Adobe invoice, GitHub Pro receipt, Canva invoice, U-NEXT / dマガジン / 日経電子版 / 月額 or 年額 plans, etc.).\n- FALSE for any of: one-off purchases (Starbucks coffee, Uber Eats meals, single retail/EC orders, single in-app purchases that are not labeled as recurring), promotional emails / newsletters / job-board digests, generic transactional confirmations that aren't recurring (shipping notifications, password resets), and *critically* BANK or CARD ISSUER notifications about a third-party charge (e.g. \"ソニー銀行 WALLET ご利用のお知らせ\", \"Visa was used at SPOTIFY\", \"Your card was charged ¥980 at MERCHANT\"). The bank notification is not itself the subscription — the user tracks the merchant's own renewal email separately.\n\nWhen `is_subscription` is FALSE, return ONLY { \"is_subscription\": false } — omit every other field. Downstream code will skip the entry.\n\nWhen `is_subscription` is TRUE, also fill the structured fields below.\n\nField conventions: Use ISO 4217 currency codes (e.g. JPY, USD). For `amount_minor`, return an INTEGER in the smallest unit of the currency — yen for JPY (no decimals), cents for USD/EUR/etc — never a string. `billing_cycle` must be one of: weekly, monthly, quarterly, semi_annual, annual, custom. `charged_at` is the ISO 8601 date the charge applies to (YYYY-MM-DD).\n\nCRITICAL rules for missing data: If any field other than `is_subscription` is not clearly present in the text, OMIT THE PROPERTY ENTIRELY. Do NOT emit placeholder strings such as \"<UNKNOWN>\", \"unknown\", \"N/A\", \"null\", \"?\", \"-\", or empty strings — they break downstream type validation.";

fn extraction_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "is_subscription": {
                "type": "boolean",
                "description": "True only for direct recurring-subscription charges from the merchant itself. False for one-off purchases, newsletters/promos, and bank/card notifications about a third-party charge."
            },
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
        },
        "required": ["is_subscription"]
    })
}

/// Drop string properties whose value is a known "I don't know" placeholder
/// that some LLMs emit instead of honoring the omit-when-missing rule
/// (e.g. `"<UNKNOWN>"`, `"N/A"`, `""`). Strict serde deserialization rejects
/// these because they hit non-`String` fields like `Option<i64>`. We delete
/// them so the property simply doesn't exist and Option fields land as None.
///
/// Recursive so it handles array-of-objects payloads from
/// [`extract_many_from_text`].
fn sanitize_llm_response(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    fn is_placeholder(s: &str) -> bool {
        let lower = s.trim().to_ascii_lowercase();
        matches!(
            lower.as_str(),
            "" | "<unknown>"
                | "unknown"
                | "n/a"
                | "na"
                | "?"
                | "-"
                | "null"
                | "none"
                | "undefined"
        )
    }
    match v {
        Value::Object(map) => {
            let cleaned: serde_json::Map<String, Value> = map
                .into_iter()
                .filter_map(|(k, val)| {
                    if let Value::String(ref s) = val
                        && is_placeholder(s)
                    {
                        return None;
                    }
                    Some((k, sanitize_llm_response(val)))
                })
                .collect();
            Value::Object(cleaned)
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(sanitize_llm_response).collect())
        }
        other => other,
    }
}

/// Run the configured LLM provider against `content`. Returns `Ok(None)` when
/// the LLM classifies the email as *not* a recurring subscription (single-shot
/// receipts, promo mail, bank-side notifications, etc.); returns
/// `Ok(Some(hint))` when it does classify as a subscription and the fields
/// deserialize cleanly.
pub async fn extract_from_text(
    client: &LlmClient,
    content: String,
) -> Result<Option<ParsedSubscriptionHint>> {
    let req = ExtractionRequest {
        system_prompt: SYSTEM_PROMPT.into(),
        user_content: content,
        schema: extraction_schema(),
    };
    let resp: ExtractionResponse = client.extract(req).await?;
    let cleaned = sanitize_llm_response(resp.data);

    let is_sub = cleaned
        .get("is_subscription")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_sub {
        return Ok(None);
    }

    let hint: ParsedSubscriptionHint = serde_json::from_value(cleaned).map_err(Error::from)?;
    Ok(Some(hint))
}

const MANY_SYSTEM_PROMPT: &str = "You are reading bulk transaction data — typically a CSV export from a bank, card, or PayPal activity report; or multiple receipts concatenated together. Identify entries that look like recurring SUBSCRIPTION payments (same merchant + same amount + predictable cadence). Skip one-off purchases, transfers, refunds, and ATM withdrawals.\n\nFor each subscription-like entry, extract one record using the same field conventions as for a single receipt: amount_minor as an integer in smallest units (yen for JPY, cents for USD), ISO 4217 currency, billing_cycle in {weekly, monthly, quarterly, semi_annual, annual, custom}, charged_at as YYYY-MM-DD. Return all detections as a JSON array under the key 'subscriptions'.\n\nCRITICAL rules for missing data: If a field on a given entry is not clearly present, OMIT THAT PROPERTY ENTIRELY from that entry. NEVER emit placeholder strings such as \"<UNKNOWN>\", \"unknown\", \"N/A\", \"null\", \"?\", \"-\", or empty strings — they break downstream type validation.";

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
        let cleaned = sanitize_llm_response(item.clone());
        // Honour the is_subscription gate when present; bulk mode also drops
        // anything the LLM marked false.
        if cleaned
            .get("is_subscription")
            .and_then(|v| v.as_bool())
            == Some(false)
        {
            continue;
        }
        if let Ok(hint) = serde_json::from_value::<ParsedSubscriptionHint>(cleaned) {
            out.push(hint);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_drops_unknown_placeholder_strings() {
        let input = serde_json::json!({
            "service_name": "Netflix",
            "amount_minor": "<UNKNOWN>",
            "currency": "JPY",
            "billing_cycle": "N/A",
            "payment_method_hint": "",
            "charged_at": "2026-05-25"
        });
        let out = sanitize_llm_response(input);
        let obj = out.as_object().unwrap();
        assert_eq!(obj.get("service_name").unwrap(), "Netflix");
        assert!(obj.get("amount_minor").is_none());
        assert_eq!(obj.get("currency").unwrap(), "JPY");
        assert!(obj.get("billing_cycle").is_none());
        assert!(obj.get("payment_method_hint").is_none());
        assert_eq!(obj.get("charged_at").unwrap(), "2026-05-25");
    }

    #[test]
    fn sanitize_recurses_into_arrays() {
        let input = serde_json::json!({
            "subscriptions": [
                { "service_name": "A", "amount_minor": "?" },
                { "service_name": "B", "amount_minor": 1500 }
            ]
        });
        let out = sanitize_llm_response(input);
        let arr = out.get("subscriptions").unwrap().as_array().unwrap();
        assert!(arr[0].as_object().unwrap().get("amount_minor").is_none());
        assert_eq!(arr[1].get("amount_minor").unwrap(), 1500);
    }

    #[test]
    fn sanitize_keeps_real_values_untouched() {
        let input = serde_json::json!({
            "service_name": "Adobe Creative Cloud",
            "amount_minor": 980,
            "currency": "USD"
        });
        let out = sanitize_llm_response(input.clone());
        assert_eq!(out, input);
    }

    #[test]
    fn sanitized_payload_deserializes_into_hint() {
        let raw = serde_json::json!({
            "is_subscription": true,
            "service_name": "Spotify",
            "amount_minor": "<UNKNOWN>",
            "currency": "USD",
            "billing_cycle": "monthly",
            "payment_method_hint": "N/A",
            "charged_at": "2026-05-31"
        });
        let cleaned = sanitize_llm_response(raw);
        let hint: ParsedSubscriptionHint = serde_json::from_value(cleaned).unwrap();
        assert_eq!(hint.service_name.as_deref(), Some("Spotify"));
        assert!(hint.amount_minor.is_none());
        assert_eq!(hint.currency.as_deref(), Some("USD"));
        assert!(hint.payment_method_hint.is_none());
    }
}
