//! `GET /v1/keys/self` handler.
//!
//! Returns the non-secret configuration of the currently authenticated API key.
//! Allows clients to verify their effective policy without admin access.
//!
//! # Response
//!
//! ```json
//! {
//!   "id": "service-a",
//!   "enabled": true,
//!   "description": "Production service",
//!   "allowed_recipient_domains": ["example.com"],
//!   "allowed_recipients": [],
//!   "rate_limit_per_min": 30,
//!   "burst": 0,
//!   "mask_recipient": null
//! }
//! ```
//!
//! `secret` is never returned.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::{auth::AuthContext, AppState};

/// GET /v1/keys/self
///
/// Returns the non-secret configuration of the authenticated API key.
pub async fn get_key_self(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> (StatusCode, Json<Value>) {
    let cfg = state.config();
    let key = cfg.security.api_keys.iter().find(|k| k.id == auth.key_id);

    match key {
        Some(k) => {
            (
                StatusCode::OK,
                Json(json!({
                    "id":                       k.id,
                    "enabled":                  k.enabled,
                    "description":              k.description,
                    "allowed_recipient_domains": k.allowed_recipient_domains,
                    "allowed_recipients":       k.allowed_recipients,
                    "rate_limit_per_min":       k.rate_limit_per_min,
                    "burst":                    k.burst,
                    "mask_recipient":           k.mask_recipient,
                    // Effective rate (for client convenience)
                    "effective_rate_limit_per_min":
                        k.rate_limit_per_min.unwrap_or(cfg.rate_limit.per_key_per_min),
                    "effective_burst":
                        if k.burst > 0 { k.burst }
                        else { cfg.rate_limit.effective_per_key_burst() },
                })),
            )
        }
        None => {
            // Should not happen: auth extractor already validated the key.
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "code":   "internal_error",
                    "message": "Authenticated key not found in config.",
                })),
            )
        }
    }
}
