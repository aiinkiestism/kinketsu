use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct ExchangeRate {
    pub base: String,
    pub quote: String,
    pub rate: f64,
    pub fetched_at: DateTime<Utc>,
}

/// Convert an amount in minor units using `rate` (interpreted as `quote_per_base`).
#[must_use]
pub fn convert_minor(amount_minor: i64, rate: f64) -> i64 {
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let converted = (amount_minor as f64 * rate).round() as i64;
    converted
}

/// Fetch latest exchange rates from open.er-api.com (free, no auth required)
/// for the given `base` currency. Caller persists via
/// [`crate::db::exchange_rates::upsert`].
pub async fn refresh_rates(base: &str) -> crate::Result<Vec<ExchangeRate>> {
    let url = format!("https://open.er-api.com/v6/latest/{base}");
    let resp = reqwest::Client::new().get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(crate::Error::Config(format!(
            "exchange-rate API {status}: {text}"
        )));
    }
    let v: serde_json::Value = resp.json().await?;
    let result = v.pointer("/result").and_then(serde_json::Value::as_str);
    if result != Some("success") {
        return Err(crate::Error::Config(format!(
            "exchange-rate API returned non-success: {result:?}"
        )));
    }
    let rates_map = v
        .pointer("/rates")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| crate::Error::Config("exchange-rate API missing rates".into()))?;
    let now = Utc::now();
    let out = rates_map
        .iter()
        .filter_map(|(quote, rate)| {
            rate.as_f64().map(|r| ExchangeRate {
                base: base.to_string(),
                quote: quote.clone(),
                rate: r,
                fetched_at: now,
            })
        })
        .collect();
    Ok(out)
}
