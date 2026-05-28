use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub name: String,
    pub service_icon: Option<String>,
    pub plan: Option<String>,
    /// Amount in minor units of `currency` (e.g. yen, cents) to avoid float drift.
    pub amount_minor: i64,
    /// ISO 4217 code, e.g. "JPY", "USD".
    pub currency: String,
    pub billing_cycle: BillingCycle,
    pub next_billing_date: Option<NaiveDate>,
    pub started_at: Option<NaiveDate>,
    pub payment_method_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub status: SubscriptionStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BillingCycle {
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Trial,
    Paused,
    Cancelled,
}
