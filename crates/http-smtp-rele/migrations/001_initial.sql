-- Migration 001: initial schema for submission status store.
--
-- Stores metadata-only submission status records.
-- Never stores mail body, subject, raw SMTP message, attachments,
-- API keys, Authorization headers, or full recipient addresses.

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS submission_statuses (
    request_id        TEXT NOT NULL PRIMARY KEY,
    key_id            TEXT NOT NULL,
    status            TEXT NOT NULL
                        CHECK (status IN (
                            'received',
                            'rejected',
                            'smtp_submission_started',
                            'smtp_accepted',
                            'smtp_failed'
                        )),
    code              TEXT,
    message           TEXT,
    -- JSON array of lowercase domain strings, e.g. '["example.com","other.org"]'
    recipient_domains TEXT NOT NULL DEFAULT '[]',
    recipient_count   INTEGER NOT NULL DEFAULT 0
                        CHECK (recipient_count >= 0),
    -- RFC 3339 timestamps
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    expires_at        TEXT NOT NULL
);

-- TTL expiry: periodic cleanup queries by expires_at
CREATE INDEX IF NOT EXISTS idx_statuses_expires_at
    ON submission_statuses (expires_at);

-- Key-scoped lookup: GET /v1/submissions/{id} filters by key_id
CREATE INDEX IF NOT EXISTS idx_statuses_key_id
    ON submission_statuses (key_id);
