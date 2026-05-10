# RFC 093 — OpenSMTPD Localhost Relay Integration

**Status.** Implemented  
**Tracks.** Platform  
**Touches.** `docs/openbsd.md`

## Summary

Document the minimal OpenSMTPD configuration required for `http-smtp-rele` to submit mail via
SMTP to localhost, and the validation that the integration works end-to-end.

## Motivation

The relay is only useful if it can reach the SMTP server. Documenting the OpenSMTPD
configuration reduces operator error and confirms that the relay's default configuration
aligns with OpenSMTPD's defaults (AC-OBSD-005).

## Scope

- Minimum `smtpd.conf` changes for localhost SMTP relay.
- Port and listener configuration.
- Testing the integration with a sample `curl` command.

## Non-goals

- Full OpenSMTPD configuration guide (too broad).
- External relay (non-localhost) configuration.

## Design

### Minimum `smtpd.conf`

The default OpenSMTPD configuration on OpenBSD typically includes a localhost listener.
Verify that the following line (or equivalent) is present in `/etc/mail/smtpd.conf`:

```
listen on lo0
```

OpenSMTPD listens on port 25 by default. `http-smtp-rele` submits to `127.0.0.1:25` by
default. No additional SMTP configuration is typically required.

### Authentication

For localhost relay, SMTP AUTH is not required. OpenSMTPD accepts unauthenticated connections
from localhost by default.

### Testing end-to-end

```sh
# Start OpenSMTPD (if not already running)
rcctl enable smtpd
rcctl start smtpd

# Start the relay
rcctl start http_smtp_rele

# Submit a test mail
curl -X POST http://127.0.0.1:8080/v1/send \
  -H "Authorization: Bearer your-api-key-secret" \
  -H "Content-Type: application/json" \
  -d '{"to":"localuser@localhost","subject":"Test","body":"Hello"}'

# Check mail delivery
mail
```

### Verifying the relay probe

```sh
curl http://127.0.0.1:8080/readyz
# {"status":"ok"}  if OpenSMTPD is running
# {"status":"error","code":"smtp_unavailable"}  if not
```

## Documentation Changes

- Add an "OpenSMTPD Integration" section to `docs/openbsd.md`.
- Include the `smtpd.conf` verification step and the `curl` test.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-093-01 | A mail submitted via the relay reaches the local mail queue. |
| AC-093-02 | `/readyz` returns 200 when OpenSMTPD is running. |
| AC-093-03 | `docs/openbsd.md` includes OpenSMTPD configuration guidance. |

## Open Questions

None.
