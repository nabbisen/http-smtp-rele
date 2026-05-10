//! Submission status tracking — types and store abstraction.
//!
//! Implements RFC 086: metadata-only status store abstraction.
//!
//! # What this stores
//!
//! What `http-smtp-rele` *observed* during request handling and SMTP submission:
//! status, error code, recipient domains (not full addresses), timestamps.
//!
//! # What this never stores
//!
//! Mail body, subject, raw SMTP message, attachments, API keys,
//! Authorization headers, full recipient addresses, or SMTP credentials.
//!
//! # Status lifecycle
//!
//! ```text
//! received ──→ smtp_submission_started ──→ smtp_accepted  (terminal)
//!                                      └──→ smtp_failed    (terminal)
//!          └──→ rejected                                   (terminal)
//! ```
//!
//! `expired` is a lifecycle event, not a stored status value.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{config::StatusConfig, request_id::RequestId};

// ---------------------------------------------------------------------------
// Domain newtype
// ---------------------------------------------------------------------------

/// A validated email domain (the part after `@`), stored in lowercase.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Domain(String);

impl Domain {
    /// Extract the domain from a full email address.
    pub fn from_address(email: &str) -> Option<Self> {
        email.rfind('@').map(|i| Domain(email[i + 1..].to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// ErrorCode — shared with HTTP error responses (RFC 086/B-2)
// ---------------------------------------------------------------------------

/// Machine-readable error codes used in both HTTP error responses and status records.
///
/// The external representation is `snake_case` string.
/// No separate StatusStore-specific namespace exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    ValidationFailed,
    PayloadTooLarge,
    UnsupportedMediaType,
    RateLimited,
    Forbidden,
    SmtpUnavailable,
    SmtpRejected,
    InternalError,
    SubmissionNotFound,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{self:?}"));
        f.write_str(&s)
    }
}

// ---------------------------------------------------------------------------
// SubmissionStatus — state machine (RFC 086/B-1)
// ---------------------------------------------------------------------------

/// The set of observable states for a submission.
///
/// `expired` is NOT a persisted status; TTL expiry deletes the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    /// Authenticated request accepted for processing.
    Received,
    /// Rejected before SMTP (validation, rate limit, policy).
    Rejected,
    /// SMTP submission attempt started.
    SmtpSubmissionStarted,
    /// SMTP server accepted the message.
    SmtpAccepted,
    /// SMTP submission failed.
    SmtpFailed,
}

impl SubmissionStatus {
    /// Terminal states will not be updated further.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Rejected | Self::SmtpAccepted | Self::SmtpFailed)
    }
}

// ---------------------------------------------------------------------------
// SubmissionStatusRecord
// ---------------------------------------------------------------------------

/// A metadata-only record describing what `http-smtp-rele` observed.
///
/// Never contains mail body, subject, full recipient addresses, tokens, or credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionStatusRecord {
    pub request_id:        RequestId,
    pub key_id:            String,
    pub status:            SubmissionStatus,
    pub code:              Option<ErrorCode>,
    /// Safe fixed-text description only — never mail content.
    pub message:           Option<String>,
    /// Deduplicated, sorted recipient domains (no local parts).
    pub recipient_domains: Vec<Domain>,
    pub recipient_count:   u32,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
    pub expires_at:        DateTime<Utc>,
}

impl SubmissionStatusRecord {
    /// Create a new record in `Received` state.
    pub fn new(
        request_id: RequestId,
        key_id: String,
        recipient_domains: Vec<Domain>,
        recipient_count: u32,
        ttl_seconds: u64,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now
            + chrono::Duration::seconds(ttl_seconds as i64);
        Self {
            request_id,
            key_id,
            status: SubmissionStatus::Received,
            code: None,
            message: Some("Submission received.".into()),
            recipient_domains,
            recipient_count,
            created_at: now,
            updated_at: now,
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

// ---------------------------------------------------------------------------
// StatusUpdate
// ---------------------------------------------------------------------------

/// An atomic status transition applied via `StatusStore::update_status`.
pub struct StatusUpdate {
    pub status:  SubmissionStatus,
    pub code:    Option<ErrorCode>,
    /// Safe fixed-text message only.
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// StatusStore trait
// ---------------------------------------------------------------------------

/// Abstract submission status store (RFC 086).
pub trait StatusStore: Send + Sync {
    /// Insert a new record. Existing records with the same `request_id` are overwritten.
    fn put(&self, record: SubmissionStatusRecord);

    /// Apply a status update to an existing record.
    ///
    /// No-ops if the record is not found, already expired, or in a terminal state.
    fn update_status(&self, request_id: &RequestId, key_id: &str, update: StatusUpdate);

    /// Look up a record by `request_id` and `key_id`.
    ///
    /// Returns `None` if: unknown, expired, or owned by a different key.
    /// Lazy expiry: expired records are deleted on access and `None` is returned.
    fn get(&self, request_id: &RequestId, key_id: &str) -> Option<SubmissionStatusRecord>;

    /// Delete all records past their `expires_at`. Called by the background cleanup task.
    fn expire_old_records(&self);

    /// Current number of live records in the store.
    fn record_count(&self) -> usize;

    /// Apply a configuration update (SIGHUP-reloadable: `ttl_seconds`, `max_records`).
    fn reload_config(&self, config: &StatusConfig);
}

// ---------------------------------------------------------------------------
// Helper: extract recipient domains from address lists
// ---------------------------------------------------------------------------

/// Extract deduplicated, sorted domains from `to` and `cc` address lists.
pub fn recipient_domains_from(to: &[String], cc: &[String]) -> Vec<Domain> {
    let mut set = std::collections::BTreeSet::new();
    for addr in to.iter().chain(cc.iter()) {
        if let Some(d) = Domain::from_address(addr) {
            set.insert(d);
        }
    }
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_from_address() {
        let d = Domain::from_address("Alice@Example.COM").unwrap();
        assert_eq!(d.as_str(), "example.com");
    }

    #[test]
    fn domain_from_invalid_email_returns_none() {
        assert!(Domain::from_address("not-an-email").is_none());
    }

    #[test]
    fn submission_status_terminal_set() {
        assert!(!SubmissionStatus::Received.is_terminal());
        assert!(!SubmissionStatus::SmtpSubmissionStarted.is_terminal());
        assert!(SubmissionStatus::Rejected.is_terminal());
        assert!(SubmissionStatus::SmtpAccepted.is_terminal());
        assert!(SubmissionStatus::SmtpFailed.is_terminal());
    }

    #[test]
    fn recipient_domains_dedup_and_sort() {
        let to  = vec!["a@example.com".into(), "b@example.org".into()];
        let cc  = vec!["c@example.com".into()]; // duplicate domain
        let ds = recipient_domains_from(&to, &cc);
        assert_eq!(ds.len(), 2);
        assert_eq!(ds[0].as_str(), "example.com");
        assert_eq!(ds[1].as_str(), "example.org");
    }

    #[test]
    fn error_code_serializes_snake_case() {
        let s = serde_json::to_string(&ErrorCode::ValidationFailed).unwrap();
        assert_eq!(s, r#""validation_failed""#);
    }

    #[test]
    fn new_record_is_received_and_not_expired() {
        let r = SubmissionStatusRecord::new(
            RequestId::new(), "key".into(), vec![], 0, 3600
        );
        assert_eq!(r.status, SubmissionStatus::Received);
        assert!(!r.is_expired());
    }
}
