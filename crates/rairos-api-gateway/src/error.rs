//! API Error types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Authentication required")]
    Unauthorized,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Rate limit exceeded")]
    RateLimited {
        limit: u32,
        reset_at: chrono::DateTime<chrono::Utc>,
    },

    #[error("Insufficient permissions for {0}")]
    Forbidden(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Payment error: {0}")]
    PaymentError(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize, ToSchema)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize, ToSchema)]
struct ErrorDetail {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reset_at: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string()),
            ApiError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "INVALID_API_KEY", self.to_string()),
            ApiError::RateLimited { limit, reset_at } => {
                let body = serde_json::to_vec(&ErrorResponse {
                    error: ErrorDetail {
                        code: "RATE_LIMITED".to_string(),
                        message: self.to_string(),
                        limit: Some(*limit),
                        reset_at: Some(reset_at.to_rfc3339()),
                    },
                }).unwrap_or_default();
                return (StatusCode::TOO_MANY_REQUESTS, body).into_response();
            }
            ApiError::Forbidden(s) => (StatusCode::FORBIDDEN, "FORBIDDEN", s.clone()),
            ApiError::NotFound(s) => (StatusCode::NOT_FOUND, "NOT_FOUND", s.clone()),
            ApiError::ValidationError(s) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", s.clone()),
            ApiError::DatabaseError(s) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", s.clone()),
            ApiError::RedisError(s) => (StatusCode::INTERNAL_SERVER_ERROR, "REDIS_ERROR", s.clone()),
            ApiError::PaymentError(s) => (StatusCode::PAYMENT_REQUIRED, "PAYMENT_ERROR", s.clone()),
            ApiError::Internal(s) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", s.clone()),
        };

        let body = serde_json::to_vec(&ErrorResponse {
            error: ErrorDetail {
                code: code.to_string(),
                message,
                limit: None,
                reset_at: None,
            },
        }).unwrap_or_default();

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;
