//! SQLite-backed submission status store (RFC 088).
//!
//! Compiled only with `--features sqlite`.
//! Uses `rusqlite` with WAL mode and a single `Mutex<Connection>`.
//!
//! # Preconditions
//!
//! The parent directory of `db_path` must exist before startup.
//! The SQLite file is created automatically on first run.
//!
//! # Migration
//!
//! Schema version is tracked with `PRAGMA user_version`.
//! Migrations are embedded SQL files (`migrations/NNN_name.sql`) and
//! run in a single transaction.  Breaking schema changes clear all records
//! (acceptable: status data is TTL-bounded and non-critical).
//!
//! # OpenBSD pledge
//!
//! SQLite mode requires `rpath wpath cpath` in addition to `stdio inet`.
//! See `security.rs` for the updated pledge set.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use arc_swap::ArcSwap;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::cmp::Ordering;
use tracing::{error, warn};

use crate::{
    config::StatusConfig,
    metrics::Metrics,
    request_id::RequestId,
    status::{ErrorCode, StatusStore, StatusUpdate, SubmissionStatus, SubmissionStatusRecord},
};

// ---------------------------------------------------------------------------
// Schema version management
// ---------------------------------------------------------------------------

const SCHEMA_VERSION: u32 = 1;

/// Each entry: (applies_when_user_version_equals, sql_to_run).
const MIGRATIONS: &[(u32, &str)] = &[(0, include_str!("../migrations/001_initial.sql"))];

// ---------------------------------------------------------------------------
// SqliteStatusStore
// ---------------------------------------------------------------------------

/// SQLite-backed, TTL-bound status store.
///
/// Thread-safe via `Mutex<Connection>`.  SQLite WAL mode allows concurrent
/// reads in future multi-connection configurations.
impl std::fmt::Debug for SqliteStatusStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStatusStore")
            .field("records", &"<sqlite connection>")
            .finish()
    }
}

pub struct SqliteStatusStore {
    conn:   Mutex<Connection>,
    config: ArcSwap<StatusConfig>,
    metrics: Arc<Metrics>,
}

impl SqliteStatusStore {
    /// Open (or create) the SQLite database at `path` and run pending migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The parent directory does not exist.
    /// - Migrations fail (schema version mismatch, I/O error).
    pub fn open(
        path: &Path,
        config: &StatusConfig,
        metrics: Arc<Metrics>,
    ) -> Result<Arc<Self>, String> {
        // Parent directory must exist; the application does not create it.
        if let Some(dir) = path.parent() {
            if !dir.exists() {
                return Err(format!(
                    "status store directory does not exist: {}. \
                     Create it before starting the application.",
                    dir.display()
                ));
            }
        }

        let conn = Connection::open(path)
            .map_err(|e| format!("failed to open SQLite db {}: {e}", path.display()))?;

        init_connection(&conn).map_err(|e| format!("SQLite init failed: {e}"))?;
        run_migrations(&conn).map_err(|e| format!("SQLite migration failed: {e}"))?;

        Ok(Arc::new(Self {
            conn:    Mutex::new(conn),
            config:  ArcSwap::from_pointee(config.clone()),
            metrics,
        }))
    }
}

impl StatusStore for SqliteStatusStore {
    // ── put ─────────────────────────────────────────────────────────────────

    fn put(&self, record: SubmissionStatusRecord) {
        let conn    = self.conn.lock().unwrap();
        let cfg     = self.config.load();

        // Enforce max_records: evict oldest before inserting.
        evict_if_needed(&conn, cfg.max_records);

        let domains_json =
            serde_json::to_string(&record.recipient_domains).unwrap_or_else(|_| "[]".into());

        let result = conn.execute(
            "INSERT OR REPLACE INTO submission_statuses
             (request_id, key_id, status, code, message, recipient_domains,
              recipient_count, created_at, updated_at, expires_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                record.request_id.as_str(),
                record.key_id,
                status_str(&record.status),
                record.code.as_ref().map(code_str),
                record.message,
                domains_json,
                record.recipient_count as i64,
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
                record.expires_at.to_rfc3339(),
            ],
        );

        if let Err(e) = result {
            error!(error = %e, "SQLite put failed");
            return;
        }

        self.metrics.status_record_created();
    }

    // ── update_status ────────────────────────────────────────────────────────

    fn update_status(&self, request_id: &RequestId, key_id: &str, update: StatusUpdate) {
        let now  = Utc::now();
        let conn = self.conn.lock().unwrap();

        let s = status_str(&update.status);
        let c = update.code.as_ref().map(code_str);

        let rows = conn.execute(
            "UPDATE submission_statuses
             SET status = ?1, code = ?2, message = ?3, updated_at = ?4
             WHERE request_id = ?5
               AND key_id     = ?6
               AND expires_at > ?7
               AND status NOT IN ('rejected','smtp_accepted','smtp_failed')",
            params![
                s,
                c,
                update.message,
                now.to_rfc3339(),
                request_id.as_str(),
                key_id,
                now.to_rfc3339(),
            ],
        )
        .unwrap_or(0);

        if rows > 0 {
            self.metrics
                .status_transitioned(s, c.as_deref().unwrap_or("none"));
        }
    }

    // ── get ─────────────────────────────────────────────────────────────────

    fn get(&self, request_id: &RequestId, key_id: &str) -> Option<SubmissionStatusRecord> {
        let now     = Utc::now();
        let now_str = now.to_rfc3339();
        let conn    = self.conn.lock().unwrap();

        let result = conn.query_row(
            "SELECT request_id, key_id, status, code, message,
                    recipient_domains, recipient_count,
                    created_at, updated_at, expires_at
             FROM submission_statuses
             WHERE request_id = ?1 AND key_id = ?2 AND expires_at > ?3",
            params![request_id.as_str(), key_id, &now_str],
            row_to_record,
        );

        match result {
            Ok(record) => Some(record),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Lazy expiry: delete the record if it exists but has expired.
                let deleted = conn
                    .execute(
                        "DELETE FROM submission_statuses
                         WHERE request_id = ?1 AND expires_at <= ?2",
                        params![request_id.as_str(), &now_str],
                    )
                    .unwrap_or(0);
                if deleted > 0 {
                    self.metrics.status_record_expired_one();
                }
                None
            }
            Err(e) => {
                error!(error = %e, "SQLite get failed");
                None
            }
        }
    }

    // ── expire_old_records ──────────────────────────────────────────────────

    fn expire_old_records(&self) {
        let now     = Utc::now();
        let conn    = self.conn.lock().unwrap();
        let cfg     = self.config.load();

        // Step 1: delete expired records.
        let deleted = conn
            .execute(
                "DELETE FROM submission_statuses WHERE expires_at <= ?1",
                params![now.to_rfc3339()],
            )
            .unwrap_or(0);

        if deleted > 0 {
            self.metrics.status_records_expired(deleted);
        }

        // Step 2: enforce max_records by evicting oldest.
        evict_if_needed(&conn, cfg.max_records);
    }

    // ── record_count ─────────────────────────────────────────────────────────

    fn record_count(&self) -> usize {
        let conn = self.conn.lock().unwrap();
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM submission_statuses",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        self.metrics.status_set_current(count);
        count
    }

    // ── reload_config ─────────────────────────────────────────────────────────

    fn reload_config(&self, config: &StatusConfig) {
        self.config.store(Arc::new(config.clone()));
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn init_connection(conn: &Connection) -> rusqlite::Result<()> {
    // These pragmas must be set before schema creation.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(())
}

fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    let current: u32 =
        conn.pragma_query_value(None, "user_version", |r| r.get(0))?;

    match current.cmp(&SCHEMA_VERSION) {
        Ordering::Equal => return Ok(()), // already up-to-date
        Ordering::Greater => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "SQLite schema version {} is newer than supported {}. \
                 Please upgrade http-smtp-rele.",
                current, SCHEMA_VERSION
            )));
        }
        Ordering::Less => {}
    }

    if current < SCHEMA_VERSION {
        warn!(
            from_version = current,
            to_version   = SCHEMA_VERSION,
            "Status store schema migration in progress. Existing records may be cleared."
        );
    }

    for (required_from, sql) in MIGRATIONS {
        if *required_from >= current && *required_from < SCHEMA_VERSION {
            conn.execute_batch(&format!(
                "BEGIN;\n{sql}\nPRAGMA user_version = {};\nCOMMIT;",
                required_from + 1
            ))?;
        }
    }

    Ok(())
}

fn evict_if_needed(conn: &Connection, max_records: usize) {
    let count: usize = conn
        .query_row("SELECT COUNT(*) FROM submission_statuses", [], |r| r.get(0))
        .unwrap_or(0);

    if count >= max_records {
        let excess = (count - max_records).max(1);
        let _ = conn.execute(
            &format!(
                "DELETE FROM submission_statuses WHERE request_id IN (
                     SELECT request_id FROM submission_statuses
                     ORDER BY created_at ASC LIMIT {excess}
                 )"
            ),
            [],
        );
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SubmissionStatusRecord> {
    let request_id_str: String  = row.get(0)?;
    let status_s: String        = row.get(2)?;
    let code_s: Option<String>  = row.get(3)?;
    let domains_s: String       = row.get(5)?;
    let created_s: String       = row.get(7)?;
    let updated_s: String       = row.get(8)?;
    let expires_s: String       = row.get(9)?;

    let parse_ts = |s: &str| {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    };

    Ok(SubmissionStatusRecord {
        request_id:        request_id_str
            .parse()
            .unwrap_or_else(|_| RequestId::new()),
        key_id:            row.get(1)?,
        status:            parse_status(&status_s),
        code:              code_s.and_then(|s| parse_code(&s)),
        message:           row.get(4)?,
        recipient_domains: serde_json::from_str(&domains_s).unwrap_or_default(),
        recipient_count:   row.get::<_, i64>(6)? as u32,
        created_at:        parse_ts(&created_s),
        updated_at:        parse_ts(&updated_s),
        expires_at:        parse_ts(&expires_s),
    })
}

// ── Status / ErrorCode serialisation ─────────────────────────────────────────

fn status_str(s: &SubmissionStatus) -> &'static str {
    match s {
        SubmissionStatus::Received              => "received",
        SubmissionStatus::Rejected              => "rejected",
        SubmissionStatus::SmtpSubmissionStarted => "smtp_submission_started",
        SubmissionStatus::SmtpAccepted          => "smtp_accepted",
        SubmissionStatus::SmtpFailed            => "smtp_failed",
    }
}

fn parse_status(s: &str) -> SubmissionStatus {
    match s {
        "received"               => SubmissionStatus::Received,
        "rejected"               => SubmissionStatus::Rejected,
        "smtp_submission_started"=> SubmissionStatus::SmtpSubmissionStarted,
        "smtp_accepted"          => SubmissionStatus::SmtpAccepted,
        "smtp_failed"            => SubmissionStatus::SmtpFailed,
        _                        => SubmissionStatus::Received, // safe fallback
    }
}

fn code_str(c: &ErrorCode) -> String {
    serde_json::to_value(c)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

fn parse_code(s: &str) -> Option<ErrorCode> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::recipient_domains_from;

    fn test_cfg() -> StatusConfig {
        StatusConfig {
            enabled:                  true,
            store:                    "sqlite".into(),
            ttl_seconds:              3600,
            max_records:              100,
            cleanup_interval_seconds: 60,
            db_path:                  None,
        }
    }

    fn make_store() -> Arc<SqliteStatusStore> {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.into_path().join("test.db"); // tempdir cleaned when path drops
        // Use keep() equivalent: just open it; tempdir path is preserved
        SqliteStatusStore::open(&path, &test_cfg(), Arc::new(Metrics::new())).unwrap()
    }

    fn make_record(id: &RequestId, key: &str, ttl: u64) -> SubmissionStatusRecord {
        SubmissionStatusRecord::new(
            id.clone(),
            key.into(),
            recipient_domains_from(&["user@example.com".to_string()], &[]),
            1,
            ttl,
        )
    }

    #[test]
    fn open_creates_db_and_schema() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.db");
        assert!(!path.exists());
        let store = SqliteStatusStore::open(&path, &test_cfg(), Arc::new(Metrics::new()));
        assert!(store.is_ok(), "open should succeed");
        assert!(path.exists(), "db file should be created");
    }

    #[test]
    fn missing_parent_dir_returns_error() {
        let path = Path::new("/nonexistent/dir/status.db");
        let result = SqliteStatusStore::open(path, &test_cfg(), Arc::new(Metrics::new()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn put_and_get_returns_record() {
        let store = make_store();
        let id    = RequestId::new();
        store.put(make_record(&id, "key-a", 3600));
        assert!(store.get(&id, "key-a").is_some());
    }

    #[test]
    fn get_wrong_key_returns_none() {
        let store = make_store();
        let id    = RequestId::new();
        store.put(make_record(&id, "key-a", 3600));
        assert!(store.get(&id, "key-b").is_none());
    }

    #[test]
    fn expired_record_returns_none() {
        let store = make_store();
        let id    = RequestId::new();
        store.put(make_record(&id, "key-a", 0)); // TTL=0 → already expired
        assert!(store.get(&id, "key-a").is_none());
    }

    #[test]
    fn update_transitions_status() {
        let store = make_store();
        let id    = RequestId::new();
        store.put(make_record(&id, "key-a", 3600));
        store.update_status(&id, "key-a", StatusUpdate {
            status:  SubmissionStatus::SmtpAccepted,
            code:    None,
            message: Some("OK".into()),
        });
        let r = store.get(&id, "key-a").unwrap();
        assert_eq!(r.status, SubmissionStatus::SmtpAccepted);
    }

    #[test]
    fn terminal_status_not_overwritten() {
        let store = make_store();
        let id    = RequestId::new();
        store.put(make_record(&id, "key-a", 3600));
        store.update_status(&id, "key-a", StatusUpdate {
            status: SubmissionStatus::SmtpAccepted, code: None, message: None,
        });
        store.update_status(&id, "key-a", StatusUpdate {
            status: SubmissionStatus::SmtpFailed,
            code:   Some(ErrorCode::SmtpUnavailable),
            message: None,
        });
        let r = store.get(&id, "key-a").unwrap();
        assert_eq!(r.status, SubmissionStatus::SmtpAccepted, "terminal must not change");
    }

    #[test]
    fn expire_old_records_removes_expired() {
        let store = make_store();
        let id1   = RequestId::new();
        let id2   = RequestId::new();
        store.put(make_record(&id1, "key-a", 0));    // expired
        store.put(make_record(&id2, "key-a", 3600)); // valid
        store.expire_old_records();
        assert_eq!(store.record_count(), 1);
        assert!(store.get(&id2, "key-a").is_some());
    }

    #[test]
    fn max_records_enforced_on_put() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut cfg = test_cfg();
        cfg.max_records = 2;
        let store = SqliteStatusStore::open(&path, &cfg, Arc::new(Metrics::new())).unwrap();

        for _ in 0..3 {
            store.put(make_record(&RequestId::new(), "key-a", 3600));
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(store.record_count() <= 2, "must be capped at max_records");
    }

    #[test]
    fn record_contains_no_sensitive_data() {
        let store = make_store();
        let id    = RequestId::new();
        let mut r = make_record(&id, "secret-key-id", 3600);
        r.message = Some("The message was accepted.".into()); // safe fixed text
        store.put(r.clone());

        let fetched = store.get(&id, "secret-key-id").unwrap();
        let serialised = serde_json::to_string(&fetched).unwrap();
        // domain is stored; full address is not
        assert!(!serialised.contains("user@example.com"), "full address must not be stored");
        assert!(serialised.contains("example.com"),       "domain may be stored");
    }

    #[test]
    fn migration_idempotent_on_reopen() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.db");
        let cfg  = test_cfg();
        let m    = Arc::new(Metrics::new());

        let s1 = SqliteStatusStore::open(&path, &cfg, m.clone()).unwrap();
        let id = RequestId::new();
        s1.put(make_record(&id, "k", 3600));
        drop(s1);

        // Reopen — should not re-run migration or lose data
        let s2 = SqliteStatusStore::open(&path, &cfg, m).unwrap();
        assert!(s2.get(&id, "k").is_some(), "data must survive reopen");
    }
}
