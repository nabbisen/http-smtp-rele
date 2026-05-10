//! Submission status tracking — types, errors, and store abstraction.
//!
//! RFC 086: metadata-only status store abstraction.
//! RFC 813: StatusStore trait API (put_received / set_recipient_metadata / update_status / get).
//! RFC 814: get() returns Result so backend failures → 503, not 404.
//!
//! `ErrorCode` is imported from `crate::error` (RFC 838 unification).

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::error::ErrorCode;
use crate::{config::StatusConfig, request_id::RequestId};

// ---------------------------------------------------------------------------
// Domain newtype
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Domain(String);

impl Domain {
    pub fn from_address(email: &str) -> Option<Self> {
        email.rfind('@').map(|i| Domain(email[i + 1..].to_lowercase()))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
}

// ---------------------------------------------------------------------------
// SubmissionStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    Received,
    Rejected,
    SmtpSubmissionStarted,
    SmtpAccepted,
    SmtpFailed,
}

impl SubmissionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Rejected | Self::SmtpAccepted | Self::SmtpFailed)
    }
}

// ---------------------------------------------------------------------------
// SubmissionStatusRecord
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionStatusRecord {
    pub request_id:        RequestId,
    pub key_id:            String,
    pub status:            SubmissionStatus,
    pub code:              Option<ErrorCode>,
    pub message:           Option<String>,
    pub recipient_domains: Vec<Domain>,
    pub recipient_count:   u32,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
    pub expires_at:        DateTime<Utc>,
}

impl SubmissionStatusRecord {
    /// Create a new `Received` record. Recipient metadata is set separately
    /// after validation via `StatusStore::set_recipient_metadata` (RFC 813).
    pub fn new_received(
        request_id:  RequestId,
        key_id:      String,
        ttl_seconds: u64,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(ttl_seconds as i64);
        Self {
            request_id,
            key_id,
            status:            SubmissionStatus::Received,
            code:              None,
            message:           Some("Submission received.".into()),
            recipient_domains: vec![],
            recipient_count:   0,
            created_at:        now,
            updated_at:        now,
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool { Utc::now() > self.expires_at }
}

// ---------------------------------------------------------------------------
// StatusUpdate
// ---------------------------------------------------------------------------

pub struct StatusUpdate {
    pub status:  SubmissionStatus,
    pub code:    Option<ErrorCode>,
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// StatusStoreError — backend failure (RFC 814)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum StatusStoreError {
    BackendUnavailable(String),
    Corrupted(String),
}

impl fmt::Display for StatusStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable(e) => write!(f, "status store backend unavailable: {e}"),
            Self::Corrupted(e)          => write!(f, "status store data corrupted: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// StatusStore trait (RFC 813 + RFC 814)
// ---------------------------------------------------------------------------

pub trait StatusStore: Send + Sync {
    /// Insert a new `Received` record. Returns error if the backend fails.
    /// Calling with a duplicate `request_id` is a no-op (insert-only semantics).
    fn put_received(&self, record: SubmissionStatusRecord) -> Result<(), StatusStoreError>;

    /// Set recipient metadata after successful validation.
    /// No-op if the record is not found, expired, or non-terminal.
    fn set_recipient_metadata(
        &self,
        request_id:        &RequestId,
        key_id:            &str,
        recipient_domains: Vec<Domain>,
        recipient_count:   u32,
    ) -> Result<(), StatusStoreError>;

    /// Apply a status transition.
    /// No-op if the record is not found, expired, or already in a terminal state.
    fn update_status(
        &self,
        request_id: &RequestId,
        key_id:     &str,
        update:     StatusUpdate,
    ) -> Result<(), StatusStoreError>;

    /// Look up a record.
    ///
    /// - `Ok(Some(r))` — found, not expired, owned by key_id
    /// - `Ok(None)`    — not found, expired, or different key
    /// - `Err(_)`      — backend unavailable → caller should return 503
    fn get(
        &self,
        request_id: &RequestId,
        key_id:     &str,
    ) -> Result<Option<SubmissionStatusRecord>, StatusStoreError>;

    /// Delete TTL-expired records. Called by the background cleanup task.
    fn expire_old_records(&self);

    /// Current number of live records.
    fn record_count(&self) -> usize;

    /// Apply SIGHUP-reloadable config changes (ttl_seconds, max_records).
    fn reload_config(&self, config: &StatusConfig);
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

pub fn recipient_domains_from(to: &[String], cc: &[String]) -> Vec<Domain> {
    let mut set = std::collections::BTreeSet::new();
    for addr in to.iter().chain(cc.iter()) {
        if let Some(d) = Domain::from_address(addr) {
            set.insert(d);
        }
    }
    set.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_from_address() {
        let d = Domain::from_address("Alice@Example.COM").unwrap();
        assert_eq!(d.as_str(), "example.com");
    }

    #[test]
    fn submission_status_terminal_set() {
        assert!(!SubmissionStatus::Received.is_terminal());
        assert!(SubmissionStatus::Rejected.is_terminal());
        assert!(SubmissionStatus::SmtpAccepted.is_terminal());
    }

    #[test]
    fn recipient_domains_dedup() {
        let ds = recipient_domains_from(
            &["a@example.com".into(), "b@example.org".into()],
            &["c@example.com".into()],
        );
        assert_eq!(ds.len(), 2);
    }

    #[test]
    fn new_received_record_not_expired() {
        let r = SubmissionStatusRecord::new_received(RequestId::new(), "k".into(), 3600);
        assert_eq!(r.status, SubmissionStatus::Received);
        assert!(!r.is_expired());
    }
}
