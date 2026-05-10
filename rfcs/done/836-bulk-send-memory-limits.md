# RFC 836 — Bulk Send Memory Limits

**Status.** Proposed  
**Tracks.** T7 — Memory Safety / T3 — Validation  
**Touches.** `src/config.rs`, `src/api/send_bulk.rs`, `src/validation.rs`

## Problem

`send_bulk` builds all `PreparedMessage` objects in Phase 1 before submitting
any in Phase 2. Each message may include a decoded attachment payload and an
assembled `lettre::Message`. With HTML and multiple attachments, memory per
message can be several megabytes. With `max_bulk_messages = 100` and 5 MB
attachments each, a single request could require 500 MB.

There is no per-bulk-request aggregate size limit.

## Fix

### Config additions

```toml
[mail]
max_bulk_total_decoded_bytes = 52428800   # 50 MiB aggregate decoded across all messages

[smtp]
bulk_concurrency = 5   # already exists; enforce max in validation
```

```toml
[server]
# Hard cap on prepared bulk messages regardless of max_bulk_messages
# max_bulk_messages upper bound enforced at validation time
```

### Validation

In Phase 1 of `send_bulk`, accumulate total decoded attachment bytes and
reject the entire batch if `max_bulk_total_decoded_bytes` is exceeded:

```rust
let mut total_bytes: usize = 0;
for spec in &msg.attachments {
    total_bytes += decoded_len(spec);
    if total_bytes > cfg.mail.max_bulk_total_decoded_bytes {
        return Err(AppError::PayloadTooLarge(...));
    }
}
```

### Enforce `max_bulk_messages` upper bound in config validation

```rust
if cfg.mail.max_bulk_messages > 500 {
    return Err(ConfigError::Validation("max_bulk_messages must be ≤ 500".into()));
}
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-836-01 | Bulk request exceeding `max_bulk_total_decoded_bytes` → 413. |
| AC-836-02 | `max_bulk_messages` > 500 rejected at startup. |
| AC-836-03 | Memory usage is bounded under the new limits. |
