use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type, specta::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LearnedDecision {
    Allow,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct LearnedSender {
    pub sender: String,
    pub decision: LearnedDecision,
    pub updated_at: DateTime<Utc>,
}
