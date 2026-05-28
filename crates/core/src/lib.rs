//! kinketsu core library.
//!
//! Shared across the Tauri shells (desktop + mobile) and the standalone
//! Axum server. Holds data models, SQLite layer, LLM provider abstraction,
//! email/PayPal parsers, currency conversion, and iCalendar export.

pub mod currency;
pub mod db;
pub mod error;
pub mod ics;
pub mod llm;
pub mod models;
pub mod parsers;

pub use error::{Error, Result};
