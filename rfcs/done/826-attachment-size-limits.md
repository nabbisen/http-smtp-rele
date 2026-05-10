# RFC 826 — Attachment Size Limits: Pre-decode Check and Total Aggregate

**Status.** Proposed  
**Tracks.** T3 — Validation  
**Touches.** `src/validation.rs`, `src/config.rs`

## Problem

Attachments are base64-decoded before size is checked:

```rust
let decoded = base64::decode(&spec.data)?;
if decoded.len() > max_attachment_bytes { return Err(...) }
```

A malicious client can send a gigantic base64 string that allocates memory
before being rejected. There is also no aggregate size limit across multiple
attachments in a single request.

## Fix

### Pre-decode encoded-size check

```rust
let max_encoded = (max_attachment_bytes * 4 / 3) + 4; // overhead for padding
if spec.data.len() > max_encoded {
    return Err(AppError::Validation(
        "attachments[].data: encoded size exceeds limit".into()
    ));
}
```

### Aggregate total attachment size limit

```toml
[mail]
max_total_attachment_bytes = 10485760   # 10 MiB total across all attachments
```

Validation accumulates decoded sizes and rejects if the total exceeds this.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-826-01 | Oversized base64 is rejected before decode (no allocation). |
| AC-826-02 | Sum of all attachment sizes checked against `max_total_attachment_bytes`. |
| AC-826-03 | Config validation test covers both limits. |
