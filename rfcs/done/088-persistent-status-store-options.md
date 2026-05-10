# RFC 088 — Persistent Status Store Options: SQLite and Redis

**Status.** Proposed  
**Tracks.** T5 — Abuse / Audit  
**Milestone.** v0.7+ (not implemented in MVP)

## Summary

This RFC evaluates persistent or shared status store options for future versions.
SQLite is the preferred local durable option.
Redis/Valkey is the preferred shared TTL-oriented option.
A single text or JSON file must not be used as the primary status store.

## Candidate comparison

| Store | Use case | MVP | v0.7+ |
|-------|----------|-----|-------|
| Memory | Single-process short-term status | yes | default |
| SQLite | Single-host durable metadata | no | candidate |
| Redis/Valkey | Multi-instance shared, TTL | no | candidate |
| Append-only text log | Audit log | no | optional |
| Single JSON/text file | Primary status store | no | **rejected** |

## SQLite schema

```sql
CREATE TABLE submission_statuses (
    request_id TEXT PRIMARY KEY,
    key_id TEXT NOT NULL,
    status TEXT NOT NULL,
    code TEXT,
    message TEXT,
    recipient_domains TEXT NOT NULL,
    recipient_count INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX idx_expires ON submission_statuses (expires_at);
CREATE INDEX idx_key_id  ON submission_statuses (key_id);
```

WAL mode required. No mail body/subject/raw message stored.
OpenBSD: `unveil(db_path, "rwc")` required (changes pledge surface).

## Redis/Valkey

Key: `http-smtp-rele:submission:{request_id}` → JSON record, TTL required.  
Local network or Unix socket recommended. ACL/auth required.  
Degraded-mode behavior when Redis unavailable must be defined.

## Why single text/JSON file is rejected

Unsafe under concurrent web access; no atomic update; no efficient key lookup;
no TTL cleanup mechanism; vulnerable to partial writes.
Text files may be used as append-only audit logs only.

## Stuck-state handling for persistent stores

Non-terminal records (`received`, `smtp_submission_started`) that are older than TTL
must be cleaned up. Persistent stores must define a recovery policy for these records
that in-memory store defers to restart.
