//! Key-value settings store. Values are serialized as JSON, so any
//! `Serialize + DeserializeOwned` type can be persisted.

use serde::{Serialize, de::DeserializeOwned};
use sqlx::SqlitePool;

use crate::Result;

pub async fn get<T: DeserializeOwned>(pool: &SqlitePool, key: &str) -> Result<Option<T>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    match row {
        Some((value,)) => Ok(Some(serde_json::from_str(&value)?)),
        None => Ok(None),
    }
}

pub async fn delete(pool: &SqlitePool, key: &str) -> Result<()> {
    sqlx::query("DELETE FROM settings WHERE key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set<T: Serialize>(pool: &SqlitePool, key: &str, value: &T) -> Result<()> {
    let json = serde_json::to_string(value)?;
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE
         SET value = excluded.value,
             updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(json)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Well-known setting keys.
pub mod keys {
    pub const LLM_CONFIG: &str = "llm.config";
    pub const GMAIL_OAUTH_CREDS: &str = "gmail.oauth_credentials";
    pub const GMAIL_TOKENS: &str = "gmail.tokens";
}
