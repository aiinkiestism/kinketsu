use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A piece of evidence — usually a parsed email or PayPal subscription record —
/// that may correspond to a subscription. Sits in a review queue until the user
/// confirms or rejects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionEvent {
    pub id: Uuid,
    pub source: DetectionSource,
    /// Stable external reference, e.g. Gmail message-id. Full message body is never stored.
    pub source_ref: Option<String>,
    pub raw_summary: Option<String>,
    pub parsed_payload: serde_json::Value,
    pub confidence: f32,
    pub status: DetectionStatus,
    pub matched_subscription_id: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetectionSource {
    Gmail,
    Paypal,
    CsvImport,
    Manual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetectionStatus {
    Pending,
    Confirmed,
    Rejected,
    Duplicate,
}
