//! Senders the user has approved or rejected in the inbox. Used to short-
//! circuit the scan loop — blocked senders never reach the LLM.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::Result;
use crate::models::{LearnedDecision, LearnedSender};

pub async fn list_all(pool: &SqlitePool) -> Result<Vec<LearnedSender>> {
    let rows = sqlx::query_as::<_, LearnedSender>(
        "SELECT sender, decision, updated_at FROM learned_senders ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_blocked(pool: &SqlitePool) -> Result<Vec<LearnedSender>> {
    let rows = sqlx::query_as::<_, LearnedSender>(
        "SELECT sender, decision, updated_at FROM learned_senders WHERE decision = 'block' ORDER BY sender",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find(pool: &SqlitePool, sender: &str) -> Result<Option<LearnedSender>> {
    let row = sqlx::query_as::<_, LearnedSender>(
        "SELECT sender, decision, updated_at FROM learned_senders WHERE sender = ?",
    )
    .bind(sender)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Upsert a learned sender. Later decisions override earlier ones.
pub async fn upsert(pool: &SqlitePool, sender: &str, decision: LearnedDecision) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO learned_senders (sender, decision, updated_at)
         VALUES (?, ?, ?)
         ON CONFLICT(sender) DO UPDATE SET decision = excluded.decision, updated_at = excluded.updated_at",
    )
    .bind(sender)
    .bind(decision)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, sender: &str) -> Result<()> {
    sqlx::query("DELETE FROM learned_senders WHERE sender = ?")
        .bind(sender)
        .execute(pool)
        .await?;
    Ok(())
}
