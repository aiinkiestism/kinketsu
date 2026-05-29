use sqlx::SqlitePool;
use uuid::Uuid;

use crate::Result;
use crate::models::Subscription;

pub async fn list(pool: &SqlitePool) -> Result<Vec<Subscription>> {
    let rows = sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get(pool: &SqlitePool, id: Uuid) -> Result<Option<Subscription>> {
    let row = sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn insert(pool: &SqlitePool, sub: &Subscription) -> Result<()> {
    sqlx::query(
        "INSERT INTO subscriptions (
            id, name, service_icon, plan, amount_minor, currency,
            billing_cycle, next_billing_date, started_at,
            payment_method_id, category_id, status, notes,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(sub.id)
    .bind(&sub.name)
    .bind(&sub.service_icon)
    .bind(&sub.plan)
    .bind(sub.amount_minor)
    .bind(&sub.currency)
    .bind(sub.billing_cycle)
    .bind(sub.next_billing_date)
    .bind(sub.started_at)
    .bind(sub.payment_method_id)
    .bind(sub.category_id)
    .bind(sub.status)
    .bind(&sub.notes)
    .bind(sub.created_at)
    .bind(sub.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update(pool: &SqlitePool, sub: &Subscription) -> Result<()> {
    sqlx::query(
        "UPDATE subscriptions SET
            name = ?, service_icon = ?, plan = ?, amount_minor = ?, currency = ?,
            billing_cycle = ?, next_billing_date = ?, started_at = ?,
            payment_method_id = ?, category_id = ?, status = ?, notes = ?,
            updated_at = ?
         WHERE id = ?",
    )
    .bind(&sub.name)
    .bind(&sub.service_icon)
    .bind(&sub.plan)
    .bind(sub.amount_minor)
    .bind(&sub.currency)
    .bind(sub.billing_cycle)
    .bind(sub.next_billing_date)
    .bind(sub.started_at)
    .bind(sub.payment_method_id)
    .bind(sub.category_id)
    .bind(sub.status)
    .bind(&sub.notes)
    .bind(sub.updated_at)
    .bind(sub.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM subscriptions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
