//! SQLite persistence layer.
//!
//! Public API is per-entity submodules (`subscriptions`, `payment_methods`, …)
//! plus connect/migrate helpers.

use std::path::Path;
use std::str::FromStr;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

use crate::Result;

pub mod categories;
pub mod detection_events;
pub mod exchange_rates;
pub mod payment_methods;
pub mod settings;
pub mod subscriptions;

/// Open a SQLite pool with WAL + foreign keys enabled.
///
/// Examples of `database_url`:
/// - `sqlite::memory:`
/// - `sqlite:///absolute/path/to/kinketsu.db`
pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

pub async fn connect_file(path: &Path) -> Result<SqlitePool> {
    let url = format!("sqlite://{}", path.display());
    connect(&url).await
}

/// Run all bundled migrations from `crates/core/migrations/`.
pub async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
