# http-smtp-rele

An authenticated HTTP-to-SMTP relay for service notification pipelines.

## What it does

`http-smtp-rele` accepts a JSON `POST /v1/send` request and relays it
to a configured SMTP server. It handles authentication, validation,
rate limiting, and submission tracking so your application does not need to.

```
Your app  →  POST /v1/send  →  http-smtp-rele  →  SMTP server  →  Recipient inbox
              (JSON, Bearer)      (validates,          (OpenSMTPD,
                                   rate-limits,         Postfix, ...)
                                   audits)
```

## Design philosophy

**Minimal scope.** Mail delivery is the SMTP server's job.
`http-smtp-rele` is a bridge: it authenticates callers, validates content,
and hands the message to the SMTP server. It does not queue, retry, or track
final delivery state.

**Security by default.** Authentication is required; open relaying is
impossible without explicit misconfiguration. On OpenBSD, `pledge` and `unveil`
reduce the process's capabilities to the minimum needed at each lifecycle stage.

**Observable.** Every request carries a `request_id` (opaque ULID-based
identifier). Submission status is queryable. Prometheus metrics are built in.

## Feature summary

| Feature | Detail |
|---------|--------|
| Authentication | Bearer token API keys; constant-time comparison |
| Rate limiting | Global, per-IP, per-key; token bucket |
| Validation | Allowlisted recipient domains; header injection protection |
| Submission status | `GET /v1/submissions/{id}`; memory, SQLite, or Redis backend |
| Bulk send | `POST /v1/send-bulk`; per-message results, bounded SMTP parallelism |
| Observability | Prometheus metrics; structured JSON or text logs |
| Transport | SMTP direct or sendmail pipe; optional STARTTLS/TLS |
| OpenBSD | `pledge` + `unveil` hardening; SIGHUP config reload |
| TLS | Optional HTTPS (`--features tls`); cert/key in config |

## Quick navigation

New to `http-smtp-rele`? Start with [Getting Started](./getting-started.md).

Ready to deploy? Run through the [Security Checklist](./operations/security-checklist.md).

Sending from multiple services? See [API Reference](./guides/api-reference.md).
