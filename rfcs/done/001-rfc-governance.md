# RFC 001 — RFC Directory Structure and Lifecycle Adoption

**Status.** Implemented  
**Tracks.** Governance  
**Touches.** `rfcs/`, `rfcs/README.md`, `rfcs/proposed/`, `rfcs/done/`, `rfcs/archive/`

## Summary

Adopt the RFC lifecycle policy (RFC 000) for `http-smtp-rele` using the **4-folder variant**:
`proposed/`, `done/`, `archive/`. All design decisions are recorded as RFCs, completed RFCs are
moved (never deleted), and `rfcs/README.md` is the always-up-to-date index.

## Motivation

`http-smtp-rele` is a security-sensitive relay application. Decisions about authentication, rate
limiting, sanitization, and OpenBSD hardening must be traceable — including the alternatives
that were considered and rejected. Without a written record, future contributors will re-derive
the same constraints from scratch, often missing the original security reasoning.

A lightweight RFC process gives us:
- A searchable record of *why* the code is the way it is.
- Clear signals about what is implemented vs. still under design.
- A forcing function for writing down acceptance criteria before writing code.

## Scope

- Create `rfcs/proposed/`, `rfcs/done/`, `rfcs/archive/`.
- Create `rfcs/README.md` with a state-grouped index.
- Adopt `NNN-slug.md` naming starting at `001`.
- Define the numbering bands by milestone (001–009 governance, 010–019 skeleton, …, 120–129 release).
- Document state transitions: `proposed/ → done/` on ship; `proposed/ → archive/` on withdrawal.
- Require that `rfcs/README.md` updates happen in the same commit as any RFC move.

## Non-goals

- `draft/` folder: authors work in a branch or scratch file; only reviewable RFCs go in `proposed/`.
- `accepted/` folder (5-folder variant): not adopted because designer and implementer are the same person.
- Formal review SLAs or dedicated RFC shepherd role.
- Integration with external issue trackers.

## Design

### Folder layout

```
rfcs/
  README.md           ← index; all RFCs by state
  proposed/           ← open for review; implementation has not started
    NNN-slug.md
  done/               ← shipped; historical record
    NNN-slug.md
  archive/            ← withdrawn or superseded
    NNN-slug.md
```

### RFC numbering bands

| Band    | Area                             |
|---------|----------------------------------|
| 001–009 | RFC governance                   |
| 010–019 | Project skeleton / runtime       |
| 020–029 | Configuration / policy           |
| 030–039 | HTTP API contract                |
| 040–049 | Authentication / access control  |
| 050–059 | Validation / sanitization        |
| 060–069 | Mail construction / SMTP relay   |
| 070–079 | Rate limit / abuse prevention    |
| 080–089 | Logging / observability          |
| 090–099 | OpenBSD hardening                |
| 100–109 | Integration / test harness       |
| 110–119 | Documentation / release packaging|
| 120–129 | MVP release stabilization        |

### State transitions

```
[author writes in branch]
        |
        v
  proposed/           ← open for design review; no implementation yet
        |
        |── design accepted, work ships ──▶ done/
        |
        └── not pursuing ──────────────▶ archive/
```

A `proposed/` RFC that has been partially implemented and partially deferred may be moved to
`done/` with an explicit "deferred" note in the Status field for the deferred parts. A follow-up
RFC covers the remainder.

### Status field

Each RFC carries a `**Status.**` line at the top. Its value mirrors the folder. On ship:

```markdown
**Status.** Implemented (v0.1.0)
```

On withdrawal:

```markdown
**Status.** Withdrawn — superseded by RFC NNN.
```

The folder is authoritative; the Status field is a convenience for readers who open the file
directly.

### README structure

```markdown
# http-smtp-rele RFCs

## Proposed
| ID  | Title | Milestone |
|-----|-------|-----------|
| 001 | [RFC governance](./proposed/001-rfc-governance.md) | M0 |

## Implemented
| ID  | Title | Shipped |
|-----|-------|---------|

## Archive
| ID  | Title | Reason |
|-----|-------|--------|
```

## Implementation Plan

1. Create `rfcs/proposed/`, `rfcs/done/`, `rfcs/archive/`.
2. Write `rfcs/README.md` with initial index.
3. Place RFC 001 and RFC 002 in `proposed/`.
4. Verify `scripts/check-rfcs.sh` (see RFC 003) passes.

## Test Plan

### Operational Tests

- `rfcs/proposed/`, `rfcs/done/`, `rfcs/archive/` exist.
- `rfcs/README.md` exists.
- Every RFC in `proposed/` is listed in `README.md`.
- No RFC number appears more than once across all folders.

These checks are automated by `scripts/check-rfcs.sh` (RFC 003).

## Security Considerations

None — this RFC governs documentation structure only.

## Operational Considerations

- RFC moves must include a `README.md` update in the same commit.
- Numbers are permanent; withdrawn RFC numbers are never reused.
- Do not delete any RFC file for any reason — move to `archive/` with a reason.

## Documentation Changes

- Create `rfcs/README.md`.
- This RFC itself is the first entry.

## Acceptance Criteria

- `rfcs/proposed/`, `rfcs/done/`, `rfcs/archive/` exist.
- `rfcs/README.md` is present and lists RFC 001.
- `scripts/check-rfcs.sh` exits 0 on a clean tree.
- RFC 002 (template) is in `proposed/`.

## Open Questions

None.
