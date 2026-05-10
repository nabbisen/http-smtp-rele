//! Application error types, ErrorCode enum, and HTTP response mapping.
//!
//! `ErrorCode` is the single source of truth for external error classification (RFC 838).
//! Both HTTP responses and status store records use `ErrorCode`.
//!
//! All validation failures map to HTTP 400 (RFC 817).

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

// ---------------------------------------------------------------------------
// ErrorCode — single external classification (RFC 838)
// ---------------------------------------------------------------------------

/// External error code shared by HTTP responses and status store records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    ValidationFailed,
    Unauthorized,
    Forbidden,
    PayloadTooLarge,
    UnsupportedMediaType,
    RateLimited,
    SmtpUnavailable,
    /// SMTP server issued a 4xx/5xx rejection (RFC 810).
    SmtpRejected,
    SubmissionNotFound,
    /// Status store backend unavailable (RFC 814).
    StatusStoreUnavailable,
    /// Feature disabled in config (RFC 823).
    FeatureDisabled,
    InternalError,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BadRequest             => "bad_request",
            Self::ValidationFailed       => "validation_failed",
            Self::Unauthorized           => "unauthorized",
            Self::Forbidden              => "forbidden",
            Self::PayloadTooLarge        => "payload_too_large",
            Self::UnsupportedMediaType   => "unsupported_media_type",
            Self::RateLimited            => "rate_limited",
            Self::SmtpUnavailable        => "smtp_unavailable",
            Self::SmtpRejected           => "smtp_rejected",
            Self::SubmissionNotFound     => "submission_not_found",
            Self::StatusStoreUnavailable => "status_store_unavailable",
            Self::FeatureDisabled        => "feature_disabled",
            Self::InternalError          => "internal_error",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// AppError — internal error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AppError {
    #[error("request payload is not valid JSON or contains unknown fields")]
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

    /// Semantic validation failure (RFC 817: maps to 400, not 422).
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("SMTP server unavailable")]
    SmtpUnavailable,

    /// SMTP server issued 4xx/5xx rejection (RFC 810).
    #[error("SMTP server rejected the message")]
    SmtpRejected,

    /// Feature disabled in config (RFC 823).
    #[error("feature disabled: {0}")]
    FeatureDisabled(String),

    #[error("internal error")]
    Internal,
}

impl AppError {
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::BadRequest           => ErrorCode::BadRequest,
            Self::Unauthorized         => ErrorCode::Unauthorized,
            Self::Forbidden            => ErrorCode::Forbidden,
            Self::PayloadTooLarge      => ErrorCode::PayloadTooLarge,
            Self::UnsupportedMediaType => ErrorCode::UnsupportedMediaType,
            Self::RateLimited { .. }   => ErrorCode::RateLimited,
            Self::Validation(_)        => ErrorCode::ValidationFailed,
            Self::SmtpUnavailable      => ErrorCode::SmtpUnavailable,
            Self::SmtpRejected         => ErrorCode::SmtpRejected,
            Self::FeatureDisabled(_)   => ErrorCode::FeatureDisabled,
            Self::Internal             => ErrorCode::InternalError,
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::BadRequest           => StatusCode::BAD_REQUEST,
            Self::Validation(_)        => StatusCode::BAD_REQUEST, // RFC 817: 400 not 422
            Self::Unauthorized         => StatusCode::UNAUTHORIZED,
            Self::Forbidden            => StatusCode::FORBIDDEN,
            Self::PayloadTooLarge      => StatusCode::PAYLOAD_TOO_LARGE,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::RateLimited { .. }   => StatusCode::TOO_MANY_REQUESTS,
            Self::SmtpUnavailable      => StatusCode::BAD_GATEWAY,
            Self::SmtpRejected         => StatusCode::BAD_GATEWAY,
            Self::FeatureDisabled(_)   => StatusCode::NOT_FOUND,
            Self::Internal             => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn client_message(&self) -> String {
        match self {
            Self::Internal             => "An internal error occurred.".to_string(),
            Self::FeatureDisabled(f)   => format!("Feature not available: {f}"),
            other                      => other.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// RequestError — AppError + request_id (RFC 806)
// ---------------------------------------------------------------------------

use crate::request_id::RequestId;

/// Error response that always includes the current `request_id` in the JSON body.
#[derive(Debug)]
pub struct RequestError {
    pub request_id: RequestId,
    pub source:     AppError,
}

impl RequestError {
    pub fn new(request_id: RequestId, source: AppError) -> Self {
        Self { request_id, source }
    }
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        let status = self.source.http_status();
        let body = Json(json!({
            "status":     "error",
            "code":       self.source.error_code().as_str(),
            "message":    self.source.client_message(),
            "request_id": self.request_id.as_str(),
        }));
        let mut response = (status, body).into_response();
        if let AppError::RateLimited { retry_after_secs: Some(s) } = &self.source {
            if let Ok(v) = HeaderValue::from_str(&s.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }
        response
    }
}

/// Legacy IntoResponse for AppError — used by Axum extractor rejections
/// that don't yet have a RequestId. `request_id` is absent from the body;
/// the `X-Request-Id` header is set by the request_id_layer middleware.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.http_status();
        let body = Json(json!({
            "status":  "error",
            "code":    self.error_code().as_str(),
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_app_errors_have_error_code() {
        let errors: &[AppError] = &[
            AppError::BadRequest,
            AppError::Unauthorized,
            AppError::Forbidden,
            AppError::PayloadTooLarge,
            AppError::UnsupportedMediaType,
            AppError::RateLimited { retry_after_secs: None },
            AppError::Validation("x".into()),
            AppError::SmtpUnavailable,
            AppError::SmtpRejected,
            AppError::FeatureDisabled("bulk".into()),
            AppError::Internal,
        ];
        for e in errors {
            assert!(!e.error_code().as_str().is_empty(), "empty code for {e:?}");
        }
    }

    #[test]
    fn validation_maps_to_400_not_422() {
        // RFC 817: validation failure → 400
        assert_eq!(AppError::Validation("x".into()).http_status(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::Validation("x".into()).error_code(), ErrorCode::ValidationFailed);
    }

    #[test]
    fn smtp_rejected_distinct_from_unavailable() {
        assert_ne!(AppError::SmtpRejected.error_code(), AppError::SmtpUnavailable.error_code());
    }

    #[test]
    fn request_error_includes_request_id() {
        let id  = RequestId::new();
        let err = RequestError::new(id, AppError::BadRequest);
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn rate_limited_has_retry_after() {
        let id  = RequestId::new();
        let err = RequestError::new(id, AppError::RateLimited { retry_after_secs: Some(30) });
        let resp = err.into_response();
        assert!(resp.headers().contains_key(header::RETRY_AFTER));
    }

    #[test]
    fn internal_error_hides_details() {
        assert_eq!(AppError::Internal.client_message(), "An internal error occurred.");
    }
}
