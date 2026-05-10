# RFC 117 — Release Artifact Policy

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `CHANGELOG.md`, `ROADMAP.md`, release archive naming

## Summary

Define the format for release archives, version tagging, `CHANGELOG.md` structure, and
the checklist for preparing a release.

## Design

### Archive naming

```
http-smtp-rele-v0.1.0.tar.gz
```

Contains:
- `http-smtp-rele` binary (release build)
- `examples/`
- `README.md`
- `LICENSE`
- `NOTICE`
- `CHANGELOG.md`

### `CHANGELOG.md` format (Keep a Changelog)

```markdown
# Changelog

## [Unreleased]
### Added
### Changed
### Fixed
### Security

## [0.1.0] - YYYY-MM-DD
### Added
- Initial release
...
```

### Release checklist

1. All MVP RFCs in `rfcs/done/`.
2. `cargo test` passes.
3. `scripts/check-rfcs.sh` passes.
4. `CHANGELOG.md` has `[0.1.0]` entry with date.
5. `ROADMAP.md` has `[v0.2]` section.
6. Tag `v0.1.0` in git.
7. Build release binary: `cargo build --release`.
8. Create archive: `tar czf http-smtp-rele-v0.1.0.tar.gz ...`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-117-01 | `CHANGELOG.md` follows Keep a Changelog format. |
| AC-117-02 | Release archive name includes version. |
| AC-117-03 | Release checklist is documented in `docs/testing.md` or a `RELEASING.md`. |

## Open Questions

None.
