//! Application error types and HTTP response mapping.
//!
//! All internal errors are mapped to [`AppError`] before being returned to the client.
//! No internal details (stack traces, file paths) are exposed to clients.

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// All application-level errors.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("request payload is not valid JSON")]
    BadRequest,

    #[error("authentication required")]
    Unauthorized,

    #[error("access denied")]
    Forbidden,

    #[error("request payload too large")]
    PayloadTooLarge,

    #[error("unsupported media type")]
    UnsupportedMediaType,

    #[error("rate limit exceeded")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("SMTP server unavailable")]
    SmtpUnavailable,

    #[error("internal error")]
    Internal,
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::SmtpUnavailable => StatusCode::BAD_GATEWAY,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::BadRequest => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::PayloadTooLarge => "payload_too_large",
            Self::UnsupportedMediaType => "unsupported_media_type",
            Self::RateLimited { .. } => "rate_limited",
            Self::Validation(_) => "validation_failed",
            Self::SmtpUnavailable => "smtp_unavailable",
            Self::Internal => "internal_error",
        }
    }

    /// Human-readable message safe to expose to clients.
    pub fn client_message(&self) -> String {
        match self {
            Self::Internal => "An internal error occurred.".to_string(),
            other => other.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(json!({
            "status": "error",
            "code":   self.error_code(),
            "message": self.client_message(),
        }));

        let mut response = (status, body).into_response();

        if let Self::RateLimited { retry_after_secs: Some(s) } = &self {
            if let Ok(v) = HeaderValue::from_str(&s.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_has_correct_status() {
        let e = AppError::Validation("bad field".into());
        assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn rate_limited_has_retry_after_in_response() {
        let e = AppError::RateLimited { retry_after_secs: Some(30) };
        let resp = e.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().contains_key(header::RETRY_AFTER));
    }

    #[test]
    fn internal_error_hides_details() {
        let e = AppError::Internal;
        assert_eq!(e.client_message(), "An internal error occurred.");
    }
}
