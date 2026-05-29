use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

/// TODO: fetch daily rates from an open exchange-rate API
/// (e.g. exchangerate.host, openexchangerates.org) and persist via
/// `db::exchange_rates::upsert`.
pub async fn refresh_rates(_base: &str) -> crate::Result<Vec<ExchangeRate>> {
    Err(crate::Error::Config(
        "currency::refresh_rates not yet implemented".into(),
    ))
}
