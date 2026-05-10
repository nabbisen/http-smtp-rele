# RFC 113 — Configuration Documentation

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `docs/configuration.md`

## Summary

Write `docs/configuration.md` with a full field reference for every `[section]` and field
in the TOML schema, plus the annotated example config.

## Content outline

```markdown
# Configuration Reference

## File location and permissions

## [server]
(every field: type, default, description, valid range)

## [security]
...

## [rate_limit]
...

## [mail]
...

## [smtp]
...

## [logging]
...

## [[api_keys]]
...

## Example configuration
(full annotated example)

## Dangerous settings
(require_auth=false, empty CIDRs, log_api_key=true, mask_recipient=false)
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-113-01 | Every TOML field from RFC 020 is documented. |
| AC-113-02 | Dangerous settings are highlighted with warnings. |
| AC-113-03 | Example config in docs is identical to `examples/http-smtp-rele.toml`. |

## Open Questions

None.
