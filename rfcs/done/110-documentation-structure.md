# RFC 110 — Documentation Structure

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `docs/`, `README.md`

## Summary

Define the documentation structure: what lives in `README.md` vs. `docs/`, the `docs/` file
layout and target reader personas, and the rule that `README.md` stays concise.

## Motivation

A bloated README discourages readers. A docs directory that duplicates the README confuses
contributors. Defining the split upfront ensures docs grow in the right place.

## Design

### README structure

```markdown
1. Hero section: GitHub badges + one-sentence description
2. Overview (2–3 sentences)
3. Why / When: use cases (concise bullet list)
4. Quick Start: minimum to send a test mail
5. Features / Design Notes (brief; link to docs for detail)
6. Full documentation link
```

### `docs/` layout

```
docs/
  README.md             ← mdbook SUMMARY.md equivalent; links all chapters
  getting-started.md    ← install, configure, first send
  configuration.md      ← full TOML field reference
  api.md                ← HTTP API reference
  security.md           ← open relay prevention, API key handling, etc.
  openbsd.md            ← OpenBSD deployment guide
  testing.md            ← running tests locally
  architecture.md       ← design rationale, module map (for contributors)
  faq.md                ← common questions
```

### Target reader personas

| Persona | Primary documents |
|---------|-----------------|
| First-time user | `README.md`, `getting-started.md`, `faq.md` |
| Operator | `configuration.md`, `security.md`, `openbsd.md` |
| API integrator | `api.md`, `getting-started.md` |
| Contributor / maintainer | `architecture.md`, `testing.md`, `rfcs/README.md` |

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-110-01 | `README.md` follows the six-section structure. |
| AC-110-02 | All `docs/` files listed in this RFC exist. |
| AC-110-03 | `docs/README.md` links all chapters (mdbook-compatible). |

## Open Questions

None.
