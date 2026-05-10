# RFC 101 — SMTP Stub Server

**Status.** Implemented  
**Tracks.** Testing  
**Touches.** `tests/smtp_stub.rs` or test helper module

## Summary

Implement a minimal in-process TCP listener that accepts SMTP connections and records received
messages, enabling integration tests to verify SMTP submission without a real mail server.

## Motivation

Integration tests for the send pipeline need a way to confirm that a message reaches SMTP
and check its content. A real OpenSMTPD is not available in CI. A minimal stub that speaks
just enough SMTP to accept a message provides the necessary test double (NFR-MNT-003).

## Scope

- TCP listener on a configurable port (default: 2525 in tests).
- Handles: `EHLO`, `MAIL FROM`, `RCPT TO`, `DATA`, `QUIT`.
- Records: sender, recipient, message body.
- Thread-safe access to recorded messages for test assertions.

## Design

### Minimum SMTP dialog

```
→ 220 localhost ESMTP stub
← EHLO localhost
→ 250 OK
← MAIL FROM:<sender@example.com>
→ 250 OK
← RCPT TO:<recipient@example.com>
→ 250 OK
← DATA
→ 354 Go ahead
← (message body lines)
← .
→ 250 OK
← QUIT
→ 221 Bye
```

### Stub struct

```rust
pub struct SmtpStub {
    messages: Arc<Mutex<Vec<ReceivedMessage>>>,
}

pub struct ReceivedMessage {
    pub envelope_from: String,
    pub envelope_to: String,
    pub body: String,
}

impl SmtpStub {
    /// Start the stub on the given port. Returns a handle for stopping.
    pub async fn start(port: u16) -> (Self, JoinHandle<()>) { ... }

    /// Return all messages received so far.
    pub fn messages(&self) -> Vec<ReceivedMessage> {
        self.messages.lock().unwrap().clone()
    }

    /// Assert that exactly one message was received matching the predicate.
    pub fn assert_one<F: Fn(&ReceivedMessage) -> bool>(&self, f: F) {
        let msgs = self.messages();
        assert_eq!(msgs.iter().filter(|m| f(m)).count(), 1);
    }
}
```

### Configurable rejection

The stub can be configured to respond with SMTP errors (5xx) to test error mapping:

```rust
pub struct SmtpStubConfig {
    pub reject_mail: bool,   // respond 550 to MAIL FROM
    pub close_immediately: bool,  // close connection immediately (connection refused sim)
}
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-101-01 | SMTP stub accepts a full SMTP transaction and records the message. |
| AC-101-02 | SMTP stub can be configured to return 5xx to test error handling. |
| AC-101-03 | Integration tests use the stub to verify mail submission. |

## Open Questions

None.
