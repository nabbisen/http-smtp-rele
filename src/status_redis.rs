//! Redis/Valkey-backed submission status store (RFC 722).
//!
//! Compiled only with `--features redis`.
//!
//! # Key schema
//!
//! ```text
//! Key:   rele:s:{request_id}
//! Value: JSON of SubmissionStatusRecord
//! TTL:   set on PUT, refreshed on UPDATE
//! ```
//!
//! # Degraded-mode behaviour
//!
//! Redis unavailability does not fail mail delivery.  All write failures are
//! logged as WARN; read failures return `None` (→ 404 on status lookup).
//!
//! # max_records
//!
//! Not enforced by this store.  Configure `maxmemory-policy` in Redis/Valkey
//! (`allkeys-lru` or `volatile-lru`) to bound memory usage.
//!
//! # expire_old_records
//!
//! No-op.  Redis handles TTL expiry natively via `EXPIRE`.

use std::sync::Arc;

use arc_swap::ArcSwap;
use redis::Commands;

use crate::{
    config::StatusConfig,
    metrics::Metrics,
    request_id::RequestId,
    status::{Domain, StatusStore, StatusStoreError, StatusUpdate, SubmissionStatus, SubmissionStatusRecord},
};

const KEY_PREFIX: &str = "rele:s:";

// ---------------------------------------------------------------------------
// RedisStatusStore
// ---------------------------------------------------------------------------

/// Redis/Valkey-backed, TTL-managed, shareable status store.
pub struct RedisStatusStore {
    client:  redis::Client,
    config:  ArcSwap<StatusConfig>,
    metrics: Arc<Metrics>,
}

impl RedisStatusStore {
    pub fn open(
        redis_url: &str,
        config:    &StatusConfig,
        metrics:   Arc<Metrics>,
    ) -> Result<Arc<Self>, String> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| format!("failed to open Redis client for {redis_url}: {e}"))?;

        // Verify connectivity at startup.
        let mut conn = client.get_connection()
            .map_err(|e| format!("failed to connect to Redis at {redis_url}: {e}"))?;
        let _: String = redis::cmd("PING").query(&mut conn)
            .map_err(|e| format!("Redis PING failed: {e}"))?;

        tracing::info!(url = redis_url, "Redis status store connected");
        Ok(Arc::new(Self {
            client,
            config: ArcSwap::from_pointee(config.clone()),
            metrics,
        }))
    }

    fn get_conn(&self) -> Option<redis::Connection> {
        match self.client.get_connection() {
            Ok(c)  => Some(c),
            Err(e) => {
                tracing::warn!(error = %e, "Redis connection failed (degraded mode)");
                None
            }
        }
    }

    fn key(request_id: &RequestId) -> String {
        format!("{KEY_PREFIX}{}", request_id.as_str())
    }
}

impl StatusStore for RedisStatusStore {
    // ── put ─────────────────────────────────────────────────────────────────

    fn put_received(&self, record: SubmissionStatusRecord) -> Result<(), StatusStoreError> {
        let cfg = self.config.load();
        let k   = Self::key(&record.request_id);
        let ttl: u64 = cfg.ttl_seconds;
        // Insert-only: check if key already exists
        let Some(mut conn) = self.get_conn() else { return Ok(()); };
        let exists: bool = conn.exists(&k).unwrap_or(false);
        if exists { return Ok(()); }

        let json = match serde_json::to_string(&record) {
            Ok(j)  => j,
            Err(e) => {
                tracing::warn!(error = %e, "Redis put: serialisation failed");
                return Err(StatusStoreError::BackendUnavailable("serialisation failed".into()));
            }
        };

        if let Err(e) = conn.set_ex::<_, _, ()>(&k, &json, ttl) {
            tracing::warn!(error = %e, key = %k, "Redis SET EX failed");
            return Err(StatusStoreError::BackendUnavailable(e.to_string()));
        }
        self.metrics.status_record_created();
        Ok(())
    }

    fn set_recipient_metadata(
        &self, request_id: &RequestId, key_id: &str,
        recipient_domains: Vec<Domain>, recipient_count: u32,
    ) -> Result<(), StatusStoreError> {
        let k = Self::key(request_id);
        let Some(mut conn) = self.get_conn() else { return Ok(()); };
        let raw: Option<String> = conn.get(&k).unwrap_or(None);
        let Some(raw) = raw else { return Ok(()); };
        if let Ok(mut record) = serde_json::from_str::<SubmissionStatusRecord>(&raw) {
            if record.key_id == key_id && !record.is_expired() {
                record.recipient_domains = recipient_domains;
                record.recipient_count   = recipient_count;
                record.updated_at        = chrono::Utc::now();
                let cfg = self.config.load();
                let remaining = record.expires_at
                    .signed_duration_since(chrono::Utc::now())
                    .num_seconds().max(1) as u64;
                if let Ok(json) = serde_json::to_string(&record) {
                    let _ = conn.set_ex::<_, _, ()>(&k, &json, remaining);
                }
            }
        }
        Ok(())
    }

    // ── update_status ────────────────────────────────────────────────────────

    fn update_status(&self, request_id: &RequestId, key_id: &str, update: StatusUpdate) -> Result<(), StatusStoreError> {
        let k = Self::key(request_id);
        let Some(mut conn) = self.get_conn() else { return Ok(()); };

        // Fetch current record
        let raw: Option<String> = match conn.get(&k) {
            Ok(v)  => v,
            Err(e) => {
                tracing::warn!(error = %e, "Redis GET failed in update_status");
                return Ok(());
            }
        };

        let Some(raw) = raw else { return Ok(()); };
        let mut record: SubmissionStatusRecord = match serde_json::from_str(&raw) {
            Ok(r)  => r,
            Err(e) => {
                tracing::warn!(error = %e, "Redis update_status: deserialisation failed");
                return Ok(());
            }
        };

        // Guard: wrong key_id, already terminal, or expired
        if record.key_id != key_id || record.is_expired() || record.status.is_terminal() {
            return Ok(());
        }

        let s = update.status;
        let c = update.code.as_ref()
            .and_then(|c| serde_json::to_value(c).ok())
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "none".into());

        record.status  = s.clone();
        record.code    = update.code;
        if update.message.is_some() { record.message = update.message; }
        record.updated_at = chrono::Utc::now();

        match serde_json::to_string(&record) {
            Ok(json) => {
                // RFC 830: use remaining TTL, not full reset
                let remaining = record.expires_at
                    .signed_duration_since(chrono::Utc::now())
                    .num_seconds().max(1) as u64;
                if let Err(e) = conn.set_ex::<_, _, ()>(&k, &json, remaining) {
                    tracing::warn!(error = %e, "Redis SET EX failed in update_status");
                } else { // ok
                    let status_s = serde_json::to_value(&s)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown".into());
                    self.metrics.status_transitioned(&status_s, &c);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Redis update_status: serialisation failed");
            }
        }
        Ok(())
    }

    // ── get ─────────────────────────────────────────────────────────────────

    fn get(&self, request_id: &RequestId, key_id: &str) -> Result<Option<SubmissionStatusRecord>, StatusStoreError> {
        let k = Self::key(request_id);
        let mut conn = match self.get_conn() {
            Some(c) => c,
            None    => return Err(StatusStoreError::BackendUnavailable("no connection".into())),
        };
        let raw: Option<String> = match conn.get(&k) {
            Ok(v)  => v,
            Err(e) => {
                self.metrics.status_store_error("get");
                return Err(StatusStoreError::BackendUnavailable(e.to_string()));
            }
        };

        let Some(raw) = raw else { return Ok(None); };
        let record: SubmissionStatusRecord = match serde_json::from_str(&raw) {
            Ok(r)  => r,
            Err(e) => return Err(StatusStoreError::Corrupted(e.to_string())),
        };
        if record.key_id != key_id { return Ok(None); }
        if record.is_expired() {
            let _: () = conn.del(&k).unwrap_or(());
            self.metrics.status_record_expired_one();
            return Ok(None);
        }
        Ok(Some(record))
    }

    // ── expire_old_records ──────────────────────────────────────────────────

    /// No-op: Redis handles expiry natively via EXPIRE.
    fn expire_old_records(&self) {
        // Intentionally empty. Redis TTL is the primary expiry mechanism.
        // The background cleanup task runs this no-op harmlessly.
    }

    // ── record_count ─────────────────────────────────────────────────────────

    fn record_count(&self) -> usize {
        let Some(mut conn) = self.get_conn() else { return 0 };
        let pattern = format!("{KEY_PREFIX}*");
        // SCAN is O(N) but non-blocking; suitable for monitoring/metrics.
        let mut cursor: u64 = 0;
        let mut total  = 0usize;
        loop {
            let result: redis::RedisResult<(u64, Vec<String>)> = redis::cmd("SCAN")
                .arg(cursor).arg("MATCH").arg(&pattern).arg("COUNT").arg(100)
                .query(&mut conn);
            match result {
                Ok((new_cursor, keys)) => {
                    total    += keys.len();
                    cursor    = new_cursor;
                    if cursor == 0 { break; }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Redis SCAN failed in record_count");
                    break;
                }
            }
        }
        self.metrics.status_set_current(total);
        total
    }

    // ── reload_config ─────────────────────────────────────────────────────────

    fn reload_config(&self, config: &StatusConfig) {
        self.config.store(Arc::new(config.clone()));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::recipient_domains_from;

    /// Returns the Redis test URL from the environment, or None to skip.
    fn redis_url() -> Option<String> {
        std::env::var("REDIS_TEST_URL").ok()
    }

    fn test_cfg() -> StatusConfig {
        StatusConfig {
            enabled:                  true,
            store:                    "redis".into(),
            ttl_seconds:              60,
            max_records:              1000,
            cleanup_interval_seconds: 60,
            db_path:                  None,
            redis_url:                None,
        }
    }

    fn make_record(id: &RequestId, key: &str, ttl: u64) -> SubmissionStatusRecord {
        SubmissionStatusRecord::new_received(id.clone(), key.into(), ttl)
    }

    // ── Serialisation unit tests (no Redis required) ─────────────────────────

    #[test]
    fn record_serialises_and_deserialises() {
        let id = RequestId::new();
        let r  = make_record(&id, "k", 3600);
        let json = serde_json::to_string(&r).unwrap();
        let r2: SubmissionStatusRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.request_id.as_str(), id.as_str());
        assert_eq!(r2.key_id, "k");
    }

    #[test]
    fn key_prefix_correct() {
        let id  = RequestId::new();
        let key = RedisStatusStore::key(&id);
        assert!(key.starts_with("rele:s:req_"), "key: {key}");
    }

    // ── Integration tests (require REDIS_TEST_URL) ───────────────────────────

    #[test]
    fn redis_put_and_get() {
        let Some(url) = redis_url() else { return };
        let store = RedisStatusStore::open(
            &url, &test_cfg(), Arc::new(Metrics::new())
        ).expect("Redis connect");
        let id = RequestId::new();
        let _ = store.put_received(make_record(&id, "key-a", 60)).unwrap();
        let r = store.get(&id, "key-a");
        assert!(r.is_some(), "record must be retrievable");
    }

    #[test]
    fn redis_wrong_key_returns_none() {
        let Some(url) = redis_url() else { return };
        let store = RedisStatusStore::open(
            &url, &test_cfg(), Arc::new(Metrics::new())
        ).unwrap();
        let id = RequestId::new();
        let _ = store.put_received(make_record(&id, "key-a", 60)).unwrap();
        assert!(store.get(&id, "key-b").unwrap().is_none());
    }

    #[test]
    fn redis_update_transitions_status() {
        let Some(url) = redis_url() else { return };
        let store = RedisStatusStore::open(
            &url, &test_cfg(), Arc::new(Metrics::new())
        ).unwrap();
        let id = RequestId::new();
        let _ = store.put_received(make_record(&id, "key-a", 60)).unwrap();
        let _ = store.update_status(&id, "key-a", StatusUpdate {
            status:  SubmissionStatus::SmtpAccepted,
            code:    None,
            message: Some("ok".into()),
        }).unwrap();
        let r = store.get(&id, "key-a").unwrap().unwrap();
        assert_eq!(r.status, SubmissionStatus::SmtpAccepted);
    }

    #[test]
    fn redis_terminal_not_overwritten() {
        let Some(url) = redis_url() else { return };
        let store = RedisStatusStore::open(
            &url, &test_cfg(), Arc::new(Metrics::new())
        ).unwrap();
        let id = RequestId::new();
        let _ = store.put_received(make_record(&id, "k", 60)).unwrap();
        let _ = store.update_status(&id, "k", StatusUpdate {
            status: SubmissionStatus::SmtpAccepted, code: None, message: None,
        }).unwrap();
        let _ = store.update_status(&id, "k", StatusUpdate {
            status:  SubmissionStatus::SmtpFailed,
            code:    Some(crate::status::ErrorCode::SmtpUnavailable),
            message: None,
        }).unwrap();
        let r = store.get(&id, "k").unwrap().unwrap();
        assert_eq!(r.status, SubmissionStatus::SmtpAccepted, "terminal must not change");
    }

    #[test]
    fn redis_expired_record_returns_none() {
        let Some(url) = redis_url() else { return };
        let store = RedisStatusStore::open(
            &url, &test_cfg(), Arc::new(Metrics::new())
        ).unwrap();
        let id = RequestId::new();
        // TTL = 0 means already expired on the application side.
        // Redis still stores for at least 1 second; use ttl=0 to trigger
        // the is_expired() check.
        let _ = store.put_received(make_record(&id, "k", 0)).unwrap();
        assert!(store.get(&id, "k").unwrap().is_none(), "expired record must return None");
    }
}
