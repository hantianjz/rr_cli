use thiserror::Error;

#[derive(Error, Debug)]
pub enum RrCliError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Authentication failed: invalid or expired token")]
    AuthError,

    #[error("Rate limited by Readwise API. Retry after {0} seconds")]
    ApiRateLimited(u64),

    #[error("Internal rate limit exceeded: max {0} requests per session")]
    RateLimitExceeded(u32),

    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
}
