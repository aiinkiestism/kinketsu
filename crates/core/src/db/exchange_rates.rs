use sqlx::SqlitePool;

use crate::Result;
use crate::currency::ExchangeRate;

pub async fn latest(
    pool: &SqlitePool,
    base: &str,
    quote: &str,
) -> Result<Option<ExchangeRate>> {
    let row = sqlx::query_as::<_, ExchangeRate>(
        "SELECT base, quote, rate, fetched_at FROM exchange_rates
         WHERE base = ? AND quote = ?
         ORDER BY fetched_at DESC
         LIMIT 1",
    )
    .bind(base)
    .bind(quote)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Return the most-recent batch of rates for the given `base` (matching the
/// single most-recent `fetched_at` timestamp). Empty when nothing has been
/// refreshed yet.
pub async fn list_latest_for_base(pool: &SqlitePool, base: &str) -> Result<Vec<ExchangeRate>> {
    let rows = sqlx::query_as::<_, ExchangeRate>(
        "SELECT base, quote, rate, fetched_at FROM exchange_rates
         WHERE base = ?
         AND fetched_at = (SELECT MAX(fetched_at) FROM exchange_rates WHERE base = ?)
         ORDER BY quote",
    )
    .bind(base)
    .bind(base)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert(pool: &SqlitePool, rate: &ExchangeRate) -> Result<()> {
    sqlx::query(
        "INSERT INTO exchange_rates (base, quote, rate, fetched_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(base, quote, fetched_at) DO UPDATE SET rate = excluded.rate",
    )
    .bind(&rate.base)
    .bind(&rate.quote)
    .bind(rate.rate)
    .bind(rate.fetched_at)
    .execute(pool)
    .await?;
    Ok(())
}
