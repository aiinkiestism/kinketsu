//! CRUD for the detection event review queue.
//!
//! `parsed_payload` is stored as TEXT (JSON-encoded). We use an intermediate
//! [`DetectionEventRow`] struct for `FromRow` decoding and convert on the
//! boundary so callers see [`DetectionEvent`] with `serde_json::Value`.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{DetectionEvent, DetectionSource, DetectionStatus};
use crate::{Error, Result};

#[derive(sqlx::FromRow)]
struct DetectionEventRow {
    id: Uuid,
    source: DetectionSource,
    source_ref: Option<String>,
    raw_summary: Option<String>,
    sender: Option<String>,
    parsed_payload: String,
    confidence: f32,
    status: DetectionStatus,
    matched_subscription_id: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<DetectionEventRow> for DetectionEvent {
    type Error = Error;
    fn try_from(r: DetectionEventRow) -> Result<Self> {
        Ok(DetectionEvent {
            id: r.id,
            source: r.source,
            source_ref: r.source_ref,
            raw_summary: r.raw_summary,
            sender: r.sender,
            parsed_payload: serde_json::from_str(&r.parsed_payload)?,
            confidence: r.confidence,
            status: r.status,
            matched_subscription_id: r.matched_subscription_id,
            reviewed_at: r.reviewed_at,
            created_at: r.created_at,
        })
    }
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<DetectionEvent>> {
    let rows = sqlx::query_as::<_, DetectionEventRow>(
        "SELECT * FROM detection_events ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(DetectionEvent::try_from).collect()
}

pub async fn list_by_status(
    pool: &SqlitePool,
    status: DetectionStatus,
) -> Result<Vec<DetectionEvent>> {
    let rows = sqlx::query_as::<_, DetectionEventRow>(
        "SELECT * FROM detection_events WHERE status = ? ORDER BY created_at DESC",
    )
    .bind(status)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(DetectionEvent::try_from).collect()
}

pub async fn get(pool: &SqlitePool, id: Uuid) -> Result<Option<DetectionEvent>> {
    let row = sqlx::query_as::<_, DetectionEventRow>("SELECT * FROM detection_events WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(DetectionEvent::try_from).transpose()
}

pub async fn insert(pool: &SqlitePool, ev: &DetectionEvent) -> Result<()> {
    let payload = serde_json::to_string(&ev.parsed_payload)?;
    sqlx::query(
        "INSERT INTO detection_events (
            id, source, source_ref, raw_summary, sender, parsed_payload,
            confidence, status, matched_subscription_id, reviewed_at, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(ev.id)
    .bind(ev.source)
    .bind(&ev.source_ref)
    .bind(&ev.raw_summary)
    .bind(&ev.sender)
    .bind(payload)
    .bind(ev.confidence)
    .bind(ev.status)
    .bind(ev.matched_subscription_id)
    .bind(ev.reviewed_at)
    .bind(ev.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Refresh the parsed fields of an existing detection in place — used when a
/// re-scan produces better data (higher peak amount, inferred cycle) for a
/// merchant that's still pending review. Leaves status / reviewed_at / the
/// original `created_at` untouched.
pub async fn update_payload(
    pool: &SqlitePool,
    id: Uuid,
    parsed_payload: &serde_json::Value,
    confidence: f32,
    raw_summary: Option<&str>,
    sender: Option<&str>,
) -> Result<()> {
    let payload = serde_json::to_string(parsed_payload)?;
    sqlx::query(
        "UPDATE detection_events
         SET parsed_payload = ?, confidence = ?, raw_summary = ?, sender = ?
         WHERE id = ?",
    )
    .bind(payload)
    .bind(confidence)
    .bind(raw_summary)
    .bind(sender)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_status(
    pool: &SqlitePool,
    id: Uuid,
    status: DetectionStatus,
    matched_subscription_id: Option<Uuid>,
) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE detection_events
         SET status = ?, matched_subscription_id = ?, reviewed_at = ?
         WHERE id = ?",
    )
    .bind(status)
    .bind(matched_subscription_id)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_source_ref(
    pool: &SqlitePool,
    source: DetectionSource,
    source_ref: &str,
) -> Result<Option<DetectionEvent>> {
    let row = sqlx::query_as::<_, DetectionEventRow>(
        "SELECT * FROM detection_events WHERE source = ? AND source_ref = ? LIMIT 1",
    )
    .bind(source)
    .bind(source_ref)
    .fetch_optional(pool)
    .await?;
    row.map(DetectionEvent::try_from).transpose()
}
