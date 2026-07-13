// File: src/error.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: Central error type for aria-cli.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AriaError {
    #[error("HTTP request to aria2 RPC failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to (de)serialize JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("aria2 RPC error (code {code}): {message}")]
    Rpc { code: i64, message: String },

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("no valid URLs to add")]
    NoValidUrls,

    #[error("download not found: {0}")]
    NotFound(String),

    #[error("clipboard error: {0}")]
    Clipboard(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AriaError>;
