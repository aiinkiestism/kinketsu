use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentMethod {
    pub id: Uuid,
    pub name: String,
    pub kind: PaymentMethodKind,
    pub last4: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodKind {
    CreditCard,
    DebitCard,
    BankAccount,
    Paypal,
    /// Carrier billing (e.g. d-barai, au PAY, SoftBank "Matomete Shiharai").
    Carrier,
    /// QR / mobile wallets (e.g. PayPay, Rakuten Pay, LINE Pay).
    Wallet,
    AppStore,
    PlayStore,
    Crypto,
    Other,
}

/// Input DTO for creating a payment method. The server fills in `id` and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPaymentMethod {
    pub name: String,
    pub kind: PaymentMethodKind,
    pub last4: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

impl NewPaymentMethod {
    #[must_use]
    pub fn into_payment_method(self) -> PaymentMethod {
        let now = Utc::now();
        PaymentMethod {
            id: Uuid::now_v7(),
            name: self.name,
            kind: self.kind,
            last4: self.last4,
            color: self.color,
            icon: self.icon,
            created_at: now,
            updated_at: now,
        }
    }
}
