//! HTTP error mapping.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use networth_application::{AccountError, NetWorthError, RepositoryError};
use serde::Serialize;

/// An error that can be rendered as an HTTP response.
#[derive(Debug)]
pub enum ApiError {
    /// The request was malformed or failed validation.
    BadRequest(String),
    /// Authentication is required or credentials are invalid.
    Unauthorized(String),
    /// Authenticated but not allowed.
    Forbidden(String),
    /// The resource does not exist.
    NotFound(String),
    /// The request conflicts with existing state.
    Conflict(String),
    /// An unexpected server-side failure.
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            ApiError::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            ApiError::Conflict(message) => (StatusCode::CONFLICT, message),
            ApiError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<AccountError> for ApiError {
    fn from(error: AccountError) -> Self {
        match error {
            AccountError::Validation(message) => ApiError::BadRequest(message),
            AccountError::CurrencyMismatch { expected, actual } => ApiError::BadRequest(format!(
                "currency mismatch: account uses {expected}, balance uses {actual}"
            )),
            AccountError::NotFound(id) => ApiError::NotFound(format!("account not found: {id}")),
            AccountError::Repository(RepositoryError::Conflict(message)) => {
                ApiError::Conflict(message)
            }
            AccountError::Repository(RepositoryError::Storage(message)) => {
                ApiError::Internal(message)
            }
        }
    }
}

impl From<NetWorthError> for ApiError {
    fn from(error: NetWorthError) -> Self {
        match error {
            NetWorthError::Validation(message) => ApiError::BadRequest(message),
            NetWorthError::Repository(RepositoryError::Conflict(message)) => {
                ApiError::Conflict(message)
            }
            NetWorthError::Repository(RepositoryError::Storage(message)) => {
                ApiError::Internal(message)
            }
        }
    }
}

impl From<networth_application::ImportError> for ApiError {
    fn from(error: networth_application::ImportError) -> Self {
        use networth_application::ImportError;
        match error {
            ImportError::Validation(message) => ApiError::BadRequest(message),
            ImportError::Repository(RepositoryError::Conflict(message)) => {
                ApiError::Conflict(message)
            }
            ImportError::Repository(RepositoryError::Storage(message)) => {
                ApiError::Internal(message)
            }
        }
    }
}

impl From<networth_application::AuthError> for ApiError {
    fn from(error: networth_application::AuthError) -> Self {
        use networth_application::AuthError;
        match error {
            AuthError::Disabled => ApiError::BadRequest("authentication is disabled".to_string()),
            AuthError::Unauthorized => {
                ApiError::Unauthorized("invalid or missing session".to_string())
            }
            AuthError::Forbidden => {
                ApiError::Forbidden("you do not have permission to do that".to_string())
            }
            AuthError::Validation(message) => ApiError::BadRequest(message),
            AuthError::Conflict(message) => ApiError::Conflict(message),
            AuthError::Internal(message) => ApiError::Internal(message),
            AuthError::Repository(RepositoryError::Conflict(message)) => {
                ApiError::Conflict(message)
            }
            AuthError::Repository(RepositoryError::Storage(message)) => ApiError::Internal(message),
        }
    }
}

impl From<networth_application::BackupError> for ApiError {
    fn from(error: networth_application::BackupError) -> Self {
        use networth_application::BackupError;
        match error {
            BackupError::Validation(message) => ApiError::BadRequest(message),
            BackupError::Repository(RepositoryError::Conflict(message)) => {
                ApiError::Conflict(message)
            }
            BackupError::Repository(RepositoryError::Storage(message)) => {
                ApiError::Internal(message)
            }
        }
    }
}
