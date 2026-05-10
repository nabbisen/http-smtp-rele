# RFC 111 — README and Quick Start

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `README.md`

## Summary

Write the `README.md` following the structure defined in RFC 110, with a Quick Start section
that allows an operator to send a test mail within five minutes.

## Design

### Quick Start content

```markdown
## Quick Start

1. Install (Linux example):
   cargo install http-smtp-rele

2. Create a config:
   cp examples/http-smtp-rele.toml /etc/http-smtp-rele.toml
   # Edit: set default_from, api_keys[].secret

3. Start:
   http-smtp-rele --config /etc/http-smtp-rele.toml

4. Send a test mail:
   curl -X POST http://127.0.0.1:8080/v1/send \
     -H "Authorization: Bearer your-secret" \
     -H "Content-Type: application/json" \
     -d '{"to":"you@example.com","subject":"Test","body":"Hello"}'
```

### Security note in README

The README must include a visible security warning:

> **Security:** `http-smtp-rele` is designed to prevent open relay. Read [docs/security.md]
> before exposing it to any network.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-111-01 | `README.md` exists and follows the structure from RFC 110. |
| AC-111-02 | Quick Start uses the example config and a `curl` command. |
| AC-111-03 | A security warning links to `docs/security.md`. |

## Open Questions

None.
