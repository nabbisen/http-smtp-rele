# Bulk Sending

`POST /v1/send-bulk` accepts an array of independent messages and returns
per-message results. Useful for notification services that send the same or
similar content to many recipients simultaneously.

---

## Request

```http
POST /v1/send-bulk
Authorization: Bearer <token>
Content-Type: application/json
```

```json
{
  "messages": [
    {
      "to": "alice@example.com",
      "subject": "Your order is ready",
      "body": "Hi Alice, your order #1234 is ready for pickup."
    },
    {
      "to": ["bob@example.com", "carol@example.com"],
      "subject": "Team update",
      "body": "Hi team, please review the attached report.",
      "html_body": "<p>Hi team, please review the attached report.</p>"
    }
  ]
}
```

Each element in `messages` has the same schema as `POST /v1/send`.

**Limit:** `[mail].max_bulk_messages` (default: `10`). Configure per deployment.

---

## Response — 202 Accepted

```json
{
  "bulk_request_id": "req_01HX...",
  "total":    2,
  "accepted": 1,
  "rejected": 1,
  "results": [
    {
      "index":      0,
      "request_id": "req_01HA...",
      "status":     "accepted"
    },
    {
      "index":      1,
      "request_id": "req_01HB...",
      "status":     "rejected",
      "code":       "validation_failed",
      "message":    "Recipient domain not allowed: example.org"
    }
  ]
}
```

- `202` is returned if auth passes and the payload structure is valid,
  regardless of individual message outcomes.
- `results` is always sorted by `index`.
- Each `results[i].request_id` is queryable via `GET /v1/submissions/`.

---

## Processing model

```
Phase 1 (sequential):
  for each message:
    rate limit check  ← per message, not per request
    validate
    build MIME message

Phase 2 (parallel, bounded by smtp.bulk_concurrency):
  submit to SMTP
  update status store
```

Rate limits are counted **per message**. A bulk request of 10 messages
consumes 10 units from each applicable bucket.

### Rate limit exhaustion mid-array

Messages processed before exhaustion keep their outcomes.
Remaining messages are rejected with `code = "rate_limited"`.

---

## Configuration

```toml
[mail]
max_bulk_messages = 10   # maximum messages per request; default 10

[smtp]
bulk_concurrency  = 5    # max parallel SMTP connections per request; 0 = unlimited
```

### Choosing `bulk_concurrency`

| Value | Behaviour |
|-------|-----------|
| `5` | Default. Safe for most SMTP servers. |
| `1` | Sequential. Use when SMTP server limits connections. |
| `0` | Unlimited. All messages submitted in parallel. |

---

## Error responses

| Condition | Status |
|-----------|--------|
| Empty `messages` array | 400 `bad_request` |
| Exceeds `max_bulk_messages` | 413 `payload_too_large` |
| Invalid auth | 401 / 403 |

---

## Example: sending with status polling

```sh
# Send bulk
RESP=$(curl -s -X POST https://relay.example.com/v1/send-bulk \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"to":"a@example.com","subject":"Hi","body":"Hello."}]}')

# Extract first message request_id
RID=$(echo "$RESP" | jq -r '.results[0].request_id')

# Poll status
curl -s "https://relay.example.com/v1/submissions/$RID" \
  -H "Authorization: Bearer $TOKEN" | jq .status
```
