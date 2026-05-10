# RFC 731 — mdbook Documentation Structure

**Status.** Proposed  
**Tracks.** T6 — Documentation  
**Touches.** `docs/`

## Summary

Organise `docs/src/` as an mdbook-compatible directory tree.
Existing content in `docs/*.md` is incorporated and expanded.
The structure follows the three reader personas defined in RFC 730.

## book.toml

```toml
[book]
title       = "http-smtp-rele"
authors     = ["nabbisen"]
language    = "en"
src         = "src"

[output.html]
git-repository-url = "https://github.com/nabbisen/http-smtp-rele"
```

## New content (not covered by existing docs)

- `src/guides/status-tracking.md` — end-to-end status tracking guide
- `src/guides/bulk-sending.md` — bulk send guide with examples
- `src/operations/reverse-proxy.md` — nginx / relayd configuration examples
- `src/development/contributing.md` — local dev setup, RFC process, PR guidelines
- `src/operations/security-checklist.md` — see RFC 732

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-731-01 | `docs/src/SUMMARY.md` is complete and internally consistent. |
| AC-731-02 | Every file referenced in SUMMARY.md exists. |
| AC-731-03 | `docs/book.toml` is present with correct `src` path. |
