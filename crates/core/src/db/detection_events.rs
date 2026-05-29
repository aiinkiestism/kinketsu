//! CRUD for the detection event review queue.
//!
//! Note: `parsed_payload` is stored as TEXT (JSON-encoded). Decoding into the
//! domain `DetectionEvent` happens at this layer, so the rest of the codebase
//! sees a `serde_json::Value` directly.
//!
//! TODO: implement once the Gmail / PayPal pipelines are landing — current
//! stubs return `not yet implemented` to keep the surface honest.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::Result;
use crate::models::{DetectionEvent, DetectionStatus};

pub async fn list(_pool: &SqlitePool) -> Result<Vec<DetectionEvent>> {
    Err(crate::Error::Config(
        "db::detection_events::list not yet implemented".into(),
    ))
}

pub async fn list_by_status(
    _pool: &SqlitePool,
    _status: DetectionStatus,
) -> Result<Vec<DetectionEvent>> {
    Err(crate::Error::Config(
        "db::detection_events::list_by_status not yet implemented".into(),
    ))
}

pub async fn insert(_pool: &SqlitePool, _ev: &DetectionEvent) -> Result<()> {
    Err(crate::Error::Config(
        "db::detection_events::insert not yet implemented".into(),
    ))
}

pub async fn update_status(
    _pool: &SqlitePool,
    _id: Uuid,
    _status: DetectionStatus,
    _matched_subscription_id: Option<Uuid>,
) -> Result<()> {
    Err(crate::Error::Config(
        "db::detection_events::update_status not yet implemented".into(),
    ))
}
