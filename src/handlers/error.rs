use std::borrow::Cow;

use axum::response::IntoResponse;
// use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("bad request: {0}, details: {1}")]
    BadRequest(String, String),
    #[error("Server error: {0}")]
    Server(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message): (http::StatusCode, Cow<'static, str>) = match self {
            ApiError::BadRequest(msg, details) => {
                tracing::warn!("{}: {}", msg, details);
                (http::StatusCode::BAD_REQUEST, Cow::Owned(msg))
            }
            ApiError::Server(msg) => {
                tracing::error!("{}", msg);
                (
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    Cow::Borrowed("Internal Server Error"),
                )
            }
            ApiError::Other(err) => {
                tracing::error!("{}", err);
                (
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    Cow::Borrowed("Internal Server Error"),
                )
            }
        };

        (status, message).into_response()
    }
}
