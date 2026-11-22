//! One error type for every handler. Handlers return `Result<T, AppError>` and
//! axum turns the error into a JSON response via `IntoResponse`.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("authentication required")]
    Unauthorized,
    #[error("not allowed")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("not enough minutes left")]
    InsufficientMinutes,
    #[error("service unavailable")]
    Unavailable,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Wire format of every error: `{"error": {"code": "...", "message": "..."}}`.
#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    /// Stable machine-readable code the app can switch on.
    pub code: String,
    /// Human-readable message; never contains internal details.
    pub message: String,
}

impl AppError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::InsufficientMinutes => (StatusCode::PAYMENT_REQUIRED, "insufficient_minutes"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            Self::Database(sqlx::Error::RowNotFound) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Database(_) | Self::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        let message = if status.is_server_error() {
            // Log the real cause; send the client a generic message only.
            tracing::error!(error = ?self, "request failed");
            match self {
                Self::Unavailable => self.to_string(),
                _ => "something went wrong".to_owned(),
            }
        } else {
            self.to_string()
        };
        let body = ErrorBody {
            error: ErrorDetail {
                code: code.to_owned(),
                message,
            },
        };
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn body_json(err: AppError) -> (StatusCode, serde_json::Value) {
        let resp = err.into_response();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn client_errors_keep_their_message() {
        let (status, body) = body_json(AppError::BadRequest("bad phone".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "bad_request");
        assert_eq!(body["error"]["message"], "bad phone");
    }

    #[tokio::test]
    async fn internal_errors_hide_details() {
        let err = AppError::Internal(anyhow::anyhow!("db password is hunter2"));
        let (status, body) = body_json(err).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"]["code"], "internal");
        assert_eq!(body["error"]["message"], "something went wrong");
    }

    #[tokio::test]
    async fn insufficient_minutes_is_402() {
        let (status, body) = body_json(AppError::InsufficientMinutes).await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(body["error"]["code"], "insufficient_minutes");
    }
}
