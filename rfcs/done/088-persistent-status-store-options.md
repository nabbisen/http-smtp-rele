# RFC 088 — Persistent Status Store Options: SQLite and Redis

**Status.** Implemented (SQLite; Redis deferred to v0.9+)  
**Tracks.** T5 — Abuse / Audit  
**Touches.** `src/status_sqlite.rs`, `src/config.rs`, `src/lib.rs`, `src/security.rs`,
             `crates/cli/src/main.rs`, `migrations/001_initial.sql`

## Summary

This RFC evaluates persistent or shared status store options.
SQLite is the preferred local durable option and is implemented in this release.
Redis/Valkey is the preferred shared TTL-oriented option and remains a future candidate.
A single text or JSON file must not be used as the primary status store.

## Design decisions (confirmed)

### User-facing switch

`[status].store` selects the backend at runtime:

```toml
[status]
store   = "memory"   # default — non-durable (RFC 087)
# store = "sqlite"   # durable — requires db_path
db_path = "/var/db/http-smtp-rele/status.db"
```

`store` and `db_path` are restart-required.  
`ttl_seconds`, `max_records`, `cleanup_interval_seconds` remain SIGHUP-reloadable.

### Optional Cargo feature

SQLite is compiled in only with `--features sqlite`:

```toml
[features]
default = []
sqlite  = ["dep:rusqlite"]
```

- Default binary has no C library dependency
- `store = "sqlite"` in a non-SQLite build → fail-fast startup error
- OpenBSD: memory-only build preserves `pledge("stdio inet")`

No `sqlx`; `rusqlite` + `spawn_blocking`-compatible sync API is sufficient
for the low-frequency, simple single-table workload.

### Database initialization

`SqliteStatusStore::open(path)` handles all initialization at startup:

1. `Connection::open(path)` — creates file if absent
2. `PRAGMA journal_mode=WAL`
3. `PRAGMA foreign_keys=ON`
4. `CREATE TABLE IF NOT EXISTS` + indexes (idempotent)
5. `PRAGMA user_version = SCHEMA_VERSION` on first creation

**Precondition:** parent directory must exist and be writable.
The application does not create the directory — that is the operator's responsibility.

### Schema migration

`PRAGMA user_version` tracks the schema version:

| Version | `user_version` | Action |
|---------|---------------|--------|
| 0 → 1  | initial setup | run `001_initial.sql` |
| N → N+1 | additive change | `ALTER TABLE ADD COLUMN` or new index |
| N → M (breaking) | table drop/recreate | **all records cleared** (logged as WARNING) |
| binary downgrade | `user_version > SCHEMA_VERSION` | **startup error** |

Status data is TTL-bounded and acceptable to lose on breaking schema changes.
Each migration runs in a transaction; partial migration is not possible.

### OpenBSD pledge impact

```
# memory store
pledge("stdio inet")

# sqlite store
pledge("stdio inet rpath wpath cpath")
unveil(db_path, "rwc")
```

SQLite mode reduces the OpenBSD hardening level. This is documented.
Memory mode is recommended for security-sensitive deployments.

## SQLite schema

```sql
CREATE TABLE submission_statuses (
    request_id       TEXT NOT NULL PRIMARY KEY,
    key_id           TEXT NOT NULL,
    status           TEXT NOT NULL,
    code             TEXT,
    message          TEXT,
    recipient_domains TEXT NOT NULL DEFAULT '[]',
    recipient_count  INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    expires_at       TEXT NOT NULL
);
CREATE INDEX idx_statuses_expires_at ON submission_statuses (expires_at);
CREATE INDEX idx_statuses_key_id     ON submission_statuses (key_id);
```

`recipient_domains` serialised as JSON array. Timestamps as RFC 3339 strings.

## Redis / Valkey (future, not implemented)

Key: `http-smtp-rele:submission:{request_id}` → JSON record, TTL required.
Local network or Unix socket recommended. ACL/auth required.
Degraded-mode behaviour when Redis is unavailable must be defined before implementation.

## Single text/JSON file — rejected

Unsafe under concurrent web access, no atomic update, no key lookup,
no TTL mechanism. May be used as append-only audit log only.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-088-01 | `store = "sqlite"` persists records across router instances. |
| AC-088-02 | Non-SQLite build rejects `store = "sqlite"` at startup. |
| AC-088-03 | Missing `db_path` with `store = "sqlite"` fails startup. |
| AC-088-04 | Missing parent directory fails startup with clear error. |
| AC-088-05 | Breaking schema change clears records and logs WARNING. |
| AC-088-06 | Downgrade (user_version > SCHEMA_VERSION) fails startup. |
| AC-088-07 | `expire_old_records()` respects `max_records`. |
| AC-088-08 | Records never contain mail body, subject, full recipient address. |
