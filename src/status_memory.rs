//! In-memory submission status store (RFC 087).
//!
//! Non-durable: all records are lost on restart.
//! Uses a hybrid cleanup strategy:
//! - Lazy expiry on `get()`.
//! - Periodic background cleanup via `expire_old_records()`.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};

use arc_swap::ArcSwap;
use chrono::Utc;

use crate::{
    config::StatusConfig,
    metrics::Metrics,
    request_id::RequestId,
    status::{StatusStore, StatusUpdate, SubmissionStatusRecord},
};

// ---------------------------------------------------------------------------
// InMemoryStatusStore
// ---------------------------------------------------------------------------

/// In-memory, TTL-bound, metadata-only status store.
///
/// Thread-safe via `RwLock`. Suitable for single-process deployments.
pub struct InMemoryStatusStore {
    /// Map from `request_id` string to record.
    records: RwLock<HashMap<String, SubmissionStatusRecord>>,
    /// Hot-swappable configuration (ttl_seconds, max_records, cleanup_interval).
    config: ArcSwap<StatusConfig>,
    /// Cumulative count of TTL-expired deletions.
    expired_total: AtomicU64,
    /// Prometheus metrics (RFC 601).
    metrics: Arc<Metrics>,
}

impl InMemoryStatusStore {
    pub fn new(config: &StatusConfig, metrics: Arc<Metrics>) -> Arc<Self> {
        Arc::new(Self {
            records: RwLock::new(HashMap::new()),
            config: ArcSwap::from_pointee(config.clone()),
            expired_total: AtomicU64::new(0),
            metrics,
        })
    }

    pub fn expired_total(&self) -> u64 {
        self.expired_total.load(Ordering::Relaxed)
    }
}

impl StatusStore for InMemoryStatusStore {
    fn put(&self, record: SubmissionStatusRecord) {
        let key = record.request_id.as_str().to_string();
        let mut map = self.records.write().unwrap();
        let cfg = self.config.load();

        // Enforce max_records: evict oldest by created_at if necessary.
        if map.len() >= cfg.max_records {
            // Find oldest non-expired entry first; if all valid, evict oldest overall.
            let to_remove = map
                .iter()
                .min_by_key(|(_, r)| r.created_at)
                .map(|(k, _)| k.clone());
            if let Some(k) = to_remove {
                map.remove(&k);
            }
        }

        map.insert(key, record);
        self.metrics.status_record_created();
    }

    fn update_status(&self, request_id: &RequestId, key_id: &str, update: StatusUpdate) {
        let mut map = self.records.write().unwrap();
        if let Some(record) = map.get_mut(request_id.as_str()) {
            // Skip if record belongs to different key, is expired, or already terminal.
            if record.key_id != key_id || record.is_expired() || record.status.is_terminal() {
                return;
            }
            let now = Utc::now();
            record.status = update.status;
            record.code = update.code;
            if update.message.is_some() {
                record.message = update.message;
            }
            record.updated_at = now;
            // Instrument transition (RFC 601)
            let status_str = serde_json::to_value(&record.status)
                .ok().and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".into());
            let code_str = record.code.as_ref()
                .and_then(|c| serde_json::to_value(c).ok())
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "none".into());
            self.metrics.status_transitioned(&status_str, &code_str);
        }
    }

    fn get(&self, request_id: &RequestId, key_id: &str) -> Option<SubmissionStatusRecord> {
        // Try read lock first.
        {
            let map = self.records.read().unwrap();
            let record = map.get(request_id.as_str())?;

            // Key mismatch → return None (same response as not-found, prevents enumeration).
            if record.key_id != key_id {
                return None;
            }

            // Still valid.
            if !record.is_expired() {
                return Some(record.clone());
            }
        }

        // Lazy expiry: upgrade to write lock and delete.
        let mut map = self.records.write().unwrap();
        if let Some(record) = map.get(request_id.as_str()) {
            if record.is_expired() {
                map.remove(request_id.as_str());
                self.expired_total.fetch_add(1, Ordering::Relaxed);
                self.metrics.status_record_expired_one();
            }
        }
        None
    }

    fn expire_old_records(&self) {
        let now = Utc::now();
        let mut map = self.records.write().unwrap();
        let cfg = self.config.load();

        // Step 1: remove expired records.
        let before = map.len();
        map.retain(|_, r| r.expires_at > now);
        let removed = before - map.len();
        if removed > 0 {
            self.expired_total.fetch_add(removed as u64, Ordering::Relaxed);
            self.metrics.status_records_expired(removed);
        }

        // Step 2: if still over max_records, evict oldest by created_at.
        if map.len() > cfg.max_records {
            let excess = map.len() - cfg.max_records;
            let mut keys_by_age: Vec<(String, chrono::DateTime<Utc>)> = map
                .iter()
                .map(|(k, r)| (k.clone(), r.created_at))
                .collect();
            keys_by_age.sort_by_key(|(_, t)| *t);
            for (k, _) in keys_by_age.into_iter().take(excess) {
                map.remove(&k);
            }
        }
    }

    fn record_count(&self) -> usize {
        let count = self.records.read().unwrap().len();
        self.metrics.status_set_current(count);
        count
    }

    fn reload_config(&self, config: &StatusConfig) {
        self.config.store(Arc::new(config.clone()));
    }
}

// ---------------------------------------------------------------------------
// NoopStatusStore
// ---------------------------------------------------------------------------

/// A no-op store used when `[status] enabled = false`.
///
/// All writes are discarded; all reads return `None`.
pub struct NoopStatusStore;

impl StatusStore for NoopStatusStore {
    fn put(&self, _record: SubmissionStatusRecord) {}
    fn update_status(&self, _: &RequestId, _: &str, _: StatusUpdate) {}
    fn get(&self, _: &RequestId, _: &str) -> Option<SubmissionStatusRecord> { None }
    fn expire_old_records(&self) {}
    fn record_count(&self) -> usize { 0 }
    fn reload_config(&self, _: &StatusConfig) {}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{ErrorCode, SubmissionStatus};

    fn test_cfg() -> StatusConfig {
        StatusConfig {
            enabled: true,
            store: "memory".into(),
            ttl_seconds: 3600,
            max_records: 100,
            cleanup_interval_seconds: 60,
            db_path: None,
        }
    }

    fn make_record(request_id: RequestId, key_id: &str, ttl: u64) -> SubmissionStatusRecord {
        use crate::status::recipient_domains_from;
        SubmissionStatusRecord::new(
            request_id, key_id.into(),
            recipient_domains_from(&["user@example.com".to_string()], &[]),
            1, ttl,
        )
    }

    #[test]
    fn put_and_get_returns_record() {
        let store = InMemoryStatusStore::new(&test_cfg(), Arc::new(Metrics::new()));
        let id = RequestId::new();
        store.put(make_record(id.clone(), "key-a", 3600));
        assert!(store.get(&id, "key-a").is_some());
    }

    #[test]
    fn get_with_wrong_key_returns_none() {
        let store = InMemoryStatusStore::new(&test_cfg(), Arc::new(Metrics::new()));
        let id = RequestId::new();
        store.put(make_record(id.clone(), "key-a", 3600));
        assert!(store.get(&id, "key-b").is_none());
    }

    #[test]
    fn expired_record_returns_none() {
        let store = InMemoryStatusStore::new(&test_cfg(), Arc::new(Metrics::new()));
        let id = RequestId::new();
        store.put(make_record(id.clone(), "key-a", 0)); // TTL = 0
        // Immediately expired
        assert!(store.get(&id, "key-a").is_none());
    }

    #[test]
    fn update_status_transitions_correctly() {
        let store = InMemoryStatusStore::new(&test_cfg(), Arc::new(Metrics::new()));
        let id = RequestId::new();
        store.put(make_record(id.clone(), "key-a", 3600));

        store.update_status(&id, "key-a", StatusUpdate {
            status: SubmissionStatus::SmtpAccepted,
            code: None,
            message: Some("accepted".into()),
        });

        let r = store.get(&id, "key-a").unwrap();
        assert_eq!(r.status, SubmissionStatus::SmtpAccepted);
    }

    #[test]
    fn terminal_status_is_not_updated() {
        let store = InMemoryStatusStore::new(&test_cfg(), Arc::new(Metrics::new()));
        let id = RequestId::new();
        store.put(make_record(id.clone(), "key-a", 3600));

        // Transition to terminal
        store.update_status(&id, "key-a", StatusUpdate {
            status: SubmissionStatus::SmtpAccepted, code: None, message: None,
        });
        // Attempt further update
        store.update_status(&id, "key-a", StatusUpdate {
            status: SubmissionStatus::SmtpFailed, code: Some(ErrorCode::SmtpUnavailable),
            message: None,
        });

        let r = store.get(&id, "key-a").unwrap();
        assert_eq!(r.status, SubmissionStatus::SmtpAccepted, "terminal status must not change");
    }

    #[test]
    fn expire_old_records_removes_expired() {
        let store = InMemoryStatusStore::new(&test_cfg(), Arc::new(Metrics::new()));
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        store.put(make_record(id1.clone(), "key-a", 0));    // expired
        store.put(make_record(id2.clone(), "key-a", 3600)); // valid

        store.expire_old_records();

        assert_eq!(store.record_count(), 1);
        assert!(store.get(&id2, "key-a").is_some());
    }

    #[test]
    fn max_records_evicts_oldest() {
        let mut cfg = test_cfg();
        cfg.max_records = 2;
        let store = InMemoryStatusStore::new(&cfg, Arc::new(Metrics::new()));

        let ids: Vec<RequestId> = (0..3).map(|_| RequestId::new()).collect();
        for id in &ids {
            store.put(make_record(id.clone(), "key-a", 3600));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        assert_eq!(store.record_count(), 2, "must be capped at max_records");
    }

    #[test]
    fn noop_store_always_returns_none() {
        let store = NoopStatusStore;
        let id = RequestId::new();
        store.put(make_record(id.clone(), "k", 3600));
        assert!(store.get(&id, "k").is_none());
        assert_eq!(store.record_count(), 0);
    }
}
