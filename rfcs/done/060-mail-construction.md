# RFC 060 — Safe Plain Text Mail Construction

**Status.** Implemented  
**Tracks.** SMTP  
**Touches.** `src/mail.rs`

## Summary

Define `mail::build_message` — the function that converts a `ValidatedMailRequest` into a
`lettre::Message`, using the library's structured builder API exclusively (no raw string
concatenation) to ensure the MIME structure is always valid and injection-safe.

## Motivation

Raw string concatenation to build email headers is the root cause of most email injection
vulnerabilities. Using `lettre::Message::builder()` delegates MIME construction and header
encoding to a well-tested library, eliminating entire classes of injection bugs
(FR-060, FR-062, FR-063).

## Scope

- `mail::build_message(req: ValidatedMailRequest, config: &MailConfig) → Result<lettre::Message, AppError>`.
- `From` header: always `config.mail.default_from` + optional `config.mail.default_from_name`.
- `To` header: `req.to`.
- `Subject` header: `req.subject` (lettre handles encoding).
- `Reply-To` header: `req.reply_to` if present.
- Plain text body: `req.body`.
- No `Cc`, `Bcc`, `X-*`, or custom headers.

## Non-goals

- HTML body (not in MVP).
- Attachments (not in MVP).
- `From` display name from request (only from config).
- Custom headers from request (never).

## Design

### `build_message`

```rust
use lettre::{
    message::{Mailbox, MultiPart, SinglePart},
    Message,
};

pub fn build_message(
    req: ValidatedMailRequest,
    config: &MailConfig,
) -> Result<Message, AppError> {
    let from_addr: lettre::Address = config
        .mail
        .default_from
        .parse()
        .map_err(|_| AppError::Internal("invalid default_from in config"))?;

    let from: Mailbox = match &config.mail.default_from_name {
        Some(name) => Mailbox::new(Some(name.clone()), from_addr),
        None => Mailbox::new(None, from_addr),
    };

    let to: Mailbox = req
        .to
        .parse()
        .map_err(|_| AppError::Internal("to address failed lettre parse post-validation"))?;

    let mut builder = Message::builder()
        .from(from)
        .to(to)
        .subject(&req.subject);

    if let Some(reply_to_str) = req.reply_to {
        let reply_to: Mailbox = reply_to_str
            .parse()
            .map_err(|_| AppError::Internal("reply_to failed lettre parse post-validation"))?;
        builder = builder.reply_to(reply_to);
    }

    builder
        .body(req.body)
        .map_err(|e| {
            tracing::error!(error = %e, "failed to build mail message");
            AppError::Internal("mail construction failed")
        })
}
```

### Security properties

- **No raw header concatenation**: all headers are set through `lettre`'s typed API.
- **`From` is always config-controlled**: the function signature does not accept a `from`
  argument. The only source of the `From` header is `config.mail.default_from`.
- **`ValidatedMailRequest` input**: by accepting only a `ValidatedMailRequest` (not a raw
  `MailRequest`), the function relies on prior validation having checked all fields.
- **lettre handles encoding**: subjects with non-ASCII characters are automatically
  encoded as RFC 2047 encoded words.

### Error mapping

`lettre::error::Error` from `builder.body(...)` maps to `AppError::Internal`. This is a
defensive mapping: by the time `build_message` is called, all inputs have been validated.
A construction failure indicates an internal inconsistency (e.g., a bug in validation).

## Test Plan

### Unit Tests

- Valid `ValidatedMailRequest` → `Ok(Message)`.
- Built message `From` header equals `default_from`.
- Built message `From` display name equals `default_from_name`.
- `Reply-To` is set when `reply_to` is `Some`.
- `Reply-To` is absent when `reply_to` is `None`.
- Message body equals `req.body`.
- Non-ASCII subject is encoded (lettre handles this; verify no panic).

### Security Tests

- The `From` address in the built message is always `config.mail.default_from`, never a
  value from the request.
- No header in the built message contains CR/LF (implied by lettre's builder; verify with
  a formatted message inspection).

## Security Considerations

- `build_message` must never accept a raw `MailRequest`. The `ValidatedMailRequest` type
  enforces this at the type system level.
- The `AppError::Internal` mapping for lettre errors means that if a construction fails after
  validation, the error is logged internally and a generic 500 is returned. The internal
  state is not exposed.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-060-01 | `From` is always `config.mail.default_from`. |
| AC-060-02 | `Reply-To` is set iff `reply_to` is `Some`. |
| AC-060-03 | No raw header string concatenation is used. |
| AC-060-04 | `build_message` accepts only `ValidatedMailRequest`, not `MailRequest`. |

## Open Questions

None.
