//! `GET /v1/submissions/{request_id}` handler.
//!
//! Implements RFC 036: metadata-only submission status lookup.
//! Access is scoped by key_id; different keys receive 404 (not 403)
//! to prevent existence enumeration.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::{
    auth::AuthContext,
    request_id::RequestId,
    status::SubmissionStatusRecord,
    AppState,
};

/// GET /v1/submissions/{request_id}
///
/// Returns the submission status record for the given `request_id`.
/// The requesting API key must be the same key that created the record.
pub async fn get_submission_status(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(raw_id): Path<String>,
) -> impl IntoResponse {
    // Parse and validate request_id format.
    let request_id: RequestId = match raw_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(not_found_body(&raw_id)),
            )
                .into_response();
        }
    };

    match state.status_store.get(&request_id, &auth.key_id) {
        Some(record) => (StatusCode::OK, Json(serialize_record(&record))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(not_found_body(request_id.as_str()))).into_response(),
    }
}

fn serialize_record(r: &SubmissionStatusRecord) -> serde_json::Value {
    json!({
        "request_id":        r.request_id,
        "status":            r.status,
        "code":              r.code,
        "message":           r.message,
        "recipient_domains": r.recipient_domains,
        "recipient_count":   r.recipient_count,
        "created_at":        r.created_at.to_rfc3339(),
        "updated_at":        r.updated_at.to_rfc3339(),
        "expires_at":        r.expires_at.to_rfc3339(),
    })
}

fn not_found_body(request_id: &str) -> serde_json::Value {
    // Generate a fresh request_id for the lookup request (correlation only).
    let lookup_id = RequestId::new();
    json!({
        "status":     "error",
        "code":       "submission_not_found",
        "message":    "Submission status was not found or has expired.",
        "request_id": request_id,
        "lookup_request_id": lookup_id,
    })
}
