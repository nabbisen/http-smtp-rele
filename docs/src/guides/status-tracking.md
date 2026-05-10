# Status Tracking

`http-smtp-rele` records what it observed during each submission so that
clients can query the outcome without polling or parsing logs.

> **Scope:** Status records describe what `http-smtp-rele` observed —
> authentication, validation, and whether the SMTP server accepted the
> message. They do not represent final delivery, bounce, or mailbox
> acknowledgement. SMTP delivery state is the configured server's responsibility.

---

## How it works

Every request to `POST /v1/send` and `POST /v1/send-bulk` receives a
`request_id` in the response. Use it with `GET /v1/submissions/{request_id}`
to query the submission outcome.

```
POST /v1/send
  ↓
request_id: req_01HX...
  ↓
GET /v1/submissions/req_01HX...
  ↓
{ "status": "smtp_accepted", ... }
```

---

## Status lifecycle

```
received
  ├─→ smtp_submission_started
  │     ├─→ smtp_accepted   (terminal ✓)
  │     └─→ smtp_failed     (terminal ✗)
  └─→ rejected              (terminal ✗)
```

| Status | Terminal | Meaning |
|--------|----------|---------|
| `received` | no | Authenticated; queued for processing |
| `smtp_submission_started` | no | SMTP connection opened |
| `smtp_accepted` | **yes** | SMTP server issued `250 OK` |
| `smtp_failed` | **yes** | SMTP submission failed |
| `rejected` | **yes** | Validation or rate limit failure |

A record that stays in a non-terminal state (e.g., after a process crash)
is retained until TTL expiry, then returns 404.

---

## Querying status

```http
GET /v1/submissions/req_01HX7Q9V6R6W9V8Y5E3E6E7M9A
Authorization: Bearer <token>
```

```json
{
  "request_id":        "req_01HX7Q9V6R6W9V8Y5E3E6E7M9A",
  "status":            "smtp_accepted",
  "code":              null,
  "message":           "The message was accepted by the configured SMTP server.",
  "recipient_domains": ["example.com"],
  "recipient_count":   1,
  "created_at":        "2026-05-10T12:00:00Z",
  "updated_at":        "2026-05-10T12:00:01Z",
  "expires_at":        "2026-05-10T13:00:00Z"
}
```

### Access control

- The same API key that sent the message must query its status.
- A different key receives `404 submission_not_found` — identical to unknown
  or expired, preventing existence enumeration.

### What is never returned

Mail body, subject, full recipient addresses, API key secrets, or SMTP
credentials are never stored and never appear in status responses.

---

## Error codes

| Code | When |
|------|------|
| `validation_failed` | Recipient domain, subject, or body failed validation |
| `payload_too_large` | Body or attachment exceeded limits |
| `rate_limited` | Global, IP, or per-key rate limit exceeded |
| `smtp_unavailable` | SMTP server unreachable or timed out |
| `smtp_rejected` | SMTP server issued a rejection response |
| `internal_error` | Unexpected internal failure |
| `submission_not_found` | Unknown, expired, or wrong-key `request_id` |

---

## Configuration

```toml
[status]
enabled                  = true
store                    = "memory"   # memory | sqlite | redis
ttl_seconds              = 3600       # record lifetime (SIGHUP-reloadable)
max_records              = 10000      # in-memory cap (SIGHUP-reloadable)
cleanup_interval_seconds = 60         # background sweep (SIGHUP-reloadable)
```

### Store options

| Store | Survives restart | Multi-instance | Notes |
|-------|-----------------|----------------|-------|
| `memory` | no | no | Default; no extra deps |
| `sqlite` | yes | no | `--features sqlite`; single host |
| `redis` | yes | **yes** | `--features redis`; shared state |

### Disabling status tracking

```toml
[status]
enabled = false
```

`request_id` is still issued and logged for correlation.
`GET /v1/submissions/` always returns 404.

---

## Bulk send status

Each message in a `POST /v1/send-bulk` request gets its own `request_id`:

```json
{
  "bulk_request_id": "req_01HX...",
  "results": [
    { "index": 0, "request_id": "req_01HA...", "status": "accepted" },
    { "index": 1, "request_id": "req_01HB...", "status": "rejected",
      "code": "validation_failed" }
  ]
}
```

Each per-message `request_id` is independently queryable via
`GET /v1/submissions/{request_id}`.

---

## Prometheus metrics

| Metric | Type | Description |
|--------|------|-------------|
| `rele_status_store_records_current` | Gauge | Live record count |
| `rele_status_store_transitions_total{status,code}` | Counter | Status changes |
| `rele_status_store_expired_total` | Counter | TTL-expired deletions |

High-cardinality labels (`request_id`, `key_id`, `recipient_domain`) are
intentionally excluded.
