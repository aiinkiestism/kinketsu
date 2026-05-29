use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub name: String,
    pub service_icon: Option<String>,
    pub plan: Option<String>,
    /// Amount in minor units of `currency` (e.g. yen, cents). Avoids float drift.
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BillingCycle {
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Trial,
    Paused,
    Cancelled,
}

/// Input DTO for creating a subscription. The server fills in `id` and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSubscription {
    pub name: String,
    pub service_icon: Option<String>,
    pub plan: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub billing_cycle: BillingCycle,
    pub next_billing_date: Option<NaiveDate>,
    pub started_at: Option<NaiveDate>,
    pub payment_method_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub status: Option<SubscriptionStatus>,
    pub notes: Option<String>,
}

impl NewSubscription {
    #[must_use]
    pub fn into_subscription(self) -> Subscription {
        let now = Utc::now();
        Subscription {
            id: Uuid::now_v7(),
            name: self.name,
            service_icon: self.service_icon,
            plan: self.plan,
            amount_minor: self.amount_minor,
            currency: self.currency,
            billing_cycle: self.billing_cycle,
            next_billing_date: self.next_billing_date,
            started_at: self.started_at,
            payment_method_id: self.payment_method_id,
            category_id: self.category_id,
            status: self.status.unwrap_or(SubscriptionStatus::Active),
            notes: self.notes,
            created_at: now,
            updated_at: now,
        }
    }
}
