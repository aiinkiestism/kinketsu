use std::path::Path;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::Result;

/// Open (or create) a SQLite pool against the given `database_url`.
///
/// Examples of `database_url`:
/// - `sqlite::memory:`
/// - `sqlite:///absolute/path/to/kinketsu.db?mode=rwc`
pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn connect_file(path: &Path) -> Result<SqlitePool> {
    let url = format!("sqlite://{}?mode=rwc", path.display());
    connect(&url).await
}
