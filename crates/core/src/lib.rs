//! Application foundations: bounded SQLite persistence and secure account sessions.
pub mod auth;
pub mod config;
pub mod database;
pub mod jobs;
pub mod listening;
pub mod network;
pub mod playlists;

use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("Authentication required")]
    Unauthorized,
    #[error("Permission denied")]
    Forbidden,
    #[error("Not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Internal(value.into())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
