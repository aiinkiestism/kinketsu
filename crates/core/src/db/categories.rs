use sqlx::SqlitePool;
use uuid::Uuid;

use crate::Result;
use crate::models::Category;

pub async fn list(pool: &SqlitePool) -> Result<Vec<Category>> {
    let rows =
        sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY name COLLATE NOCASE")
            .fetch_all(pool)
            .await?;
    Ok(rows)
}

pub async fn get(pool: &SqlitePool, id: Uuid) -> Result<Option<Category>> {
    let row = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn insert(pool: &SqlitePool, cat: &Category) -> Result<()> {
    sqlx::query(
        "INSERT INTO categories (id, name, icon, color, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(cat.id)
    .bind(&cat.name)
    .bind(&cat.icon)
    .bind(&cat.color)
    .bind(cat.created_at)
    .bind(cat.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update(pool: &SqlitePool, cat: &Category) -> Result<()> {
    sqlx::query("UPDATE categories SET name = ?, icon = ?, color = ?, updated_at = ? WHERE id = ?")
        .bind(&cat.name)
        .bind(&cat.icon)
        .bind(&cat.color)
        .bind(cat.updated_at)
        .bind(cat.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
