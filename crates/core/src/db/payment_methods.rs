use sqlx::SqlitePool;
use uuid::Uuid;

use crate::Result;
use crate::models::PaymentMethod;

pub async fn list(pool: &SqlitePool) -> Result<Vec<PaymentMethod>> {
    let rows = sqlx::query_as::<_, PaymentMethod>(
        "SELECT * FROM payment_methods ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get(pool: &SqlitePool, id: Uuid) -> Result<Option<PaymentMethod>> {
    let row = sqlx::query_as::<_, PaymentMethod>("SELECT * FROM payment_methods WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn insert(pool: &SqlitePool, pm: &PaymentMethod) -> Result<()> {
    sqlx::query(
        "INSERT INTO payment_methods (id, name, kind, last4, color, icon, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(pm.id)
    .bind(&pm.name)
    .bind(pm.kind)
    .bind(&pm.last4)
    .bind(&pm.color)
    .bind(&pm.icon)
    .bind(pm.created_at)
    .bind(pm.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update(pool: &SqlitePool, pm: &PaymentMethod) -> Result<()> {
    sqlx::query(
        "UPDATE payment_methods
         SET name = ?, kind = ?, last4 = ?, color = ?, icon = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&pm.name)
    .bind(pm.kind)
    .bind(&pm.last4)
    .bind(&pm.color)
    .bind(&pm.icon)
    .bind(pm.updated_at)
    .bind(pm.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM payment_methods WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
