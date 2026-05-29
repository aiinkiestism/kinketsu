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
    /// キャリア決済 (d払い, au PAY, ソフトバンクまとめて支払い)
    Carrier,
    /// QR/モバイルウォレット (PayPay, 楽天Pay, LINE Pay)
    Wallet,
    AppStore,
    PlayStore,
    Crypto,
    Other,
}
