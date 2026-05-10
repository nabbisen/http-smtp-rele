# RFC 002 — RFC Template and Review Checklist

**Status.** Implemented  
**Tracks.** Governance  
**Touches.** `rfcs/README.md`, all future RFCs

## Summary

Define the standard template for all `http-smtp-rele` RFCs, and a review checklist that
ensures each RFC is self-contained, testable, and security-aware before implementation begins.

## Motivation

Inconsistent RFC structure causes friction: a reviewer cannot quickly find the acceptance
criteria or the security implications. A single template gives every RFC the same skeleton,
and a review checklist gives a shared definition of "ready to implement."

## Scope

- Define the mandatory sections of every RFC.
- Define the review checklist used before an RFC is acted upon.
- Clarify which sections are required vs. optional.

## Non-goals

- Enforcing template compliance automatically (that belongs in RFC 003 / check-rfcs).
- Defining what constitutes a passing review — that is up to the maintainer.

## Design

### RFC Template

```markdown
# RFC NNN — Title

**Status.** Implemented  
**Tracks.** <Governance | Foundation | API | Security | SMTP | Ops | Platform | Release>  
**Touches.** <list of files, modules, docs, or tests this RFC affects>

## Summary

One or two sentences describing what this RFC proposes and why.

## Motivation

Why does this change need to happen? What problem does it solve?
Reference requirements (FR-xxx, NFR-xxx, AC-xxx) where applicable.

## Scope

What exactly is in scope for this RFC?
Use a concrete list; this becomes the implementation checklist.

## Non-goals

What is explicitly excluded? Why?

## Design

The main technical content. Include:
- Data structures and types
- Algorithms
- Module boundaries
- Configuration surfaces
- Error conditions
- Interactions with other modules

Use diagrams, pseudocode, or concrete Rust sketches where helpful.

## Implementation Plan

Ordered list of steps. Each step should be independently verifiable.

1. Step one.
2. Step two.

## Test Plan

### Unit Tests

List unit tests by name or description.

### Integration Tests

List integration scenarios.

### Security Tests

List security-focused tests (injection, auth bypass, secret leakage, etc.).

### Operational Tests

List startup, config, health, and shutdown tests.

## Security Considerations

What are the security implications of this RFC?
- What attack surface does it add or remove?
- Does it touch authentication, secrets, or input validation?
- What is the failure mode, and is it fail-closed?

If this RFC has no security implications, state that explicitly.

## Operational Considerations

- Does this RFC change startup behavior?
- Does it affect logging?
- Does it require config changes?
- Does it affect OpenBSD pledge/unveil?

## Documentation Changes

What documentation must be created or updated alongside this RFC?

## Acceptance Criteria

A numbered list of conditions that must all be true for this RFC to move to `done/`.
Each criterion must be independently verifiable.

| ID | Criterion |
|----|-----------|
| AC-NNN-01 | Description. |

## Open Questions

List unresolved design questions. An RFC may move to `done/` with open questions only if
the questions are deferred (with a follow-up RFC reference), not unresolved.
```

### Required sections

The following sections are **mandatory** in every RFC:

| Section | Why mandatory |
|---------|---------------|
| Summary | Quick orientation |
| Motivation | Links to requirements |
| Scope | Defines implementation boundary |
| Non-goals | Prevents scope creep |
| Design | Core technical content |
| Test Plan | Verifiability |
| Security Considerations | Must not be omitted even if empty |
| Acceptance Criteria | Definition of done |

The following sections are **optional** (include when applicable):

- Implementation Plan (helpful for complex RFCs)
- Operational Considerations
- Documentation Changes
- Open Questions

### Review checklist

Before acting on a `proposed/` RFC:

```
[ ] Summary is one or two sentences and accurately reflects the RFC.
[ ] Motivation references at least one requirement (FR, NFR, AC, or UC).
[ ] Scope is a concrete list, not vague prose.
[ ] Non-goals explicitly state what is out of scope.
[ ] Design has enough detail to begin implementation without ambiguity.
[ ] Test Plan includes at least one test for each Acceptance Criterion.
[ ] Security Considerations are present (even if "no security implications").
[ ] Acceptance Criteria are numbered and independently verifiable.
[ ] No open question blocks implementation (deferred questions are acceptable).
[ ] Status is "Proposed" and the file is in rfcs/proposed/.
[ ] File is named NNN-slug.md and listed in rfcs/README.md.
```

## Implementation Plan

1. Publish this RFC in `proposed/` alongside RFC 001.
2. Apply the template to all subsequent RFCs.
3. Use the review checklist as a gate before marking an RFC ready for implementation.

## Test Plan

### Operational Tests

- RFC 003 check script verifies that every RFC in `proposed/` has a `## Summary`, `## Scope`,
  `## Test Plan`, `## Security Considerations`, and `## Acceptance Criteria` section.

## Security Considerations

No security implications — this RFC governs documentation format only.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-002-01 | Template is published and accessible in `rfcs/proposed/002-rfc-template.md`. |
| AC-002-02 | All RFCs written after this one follow the template. |
| AC-002-03 | Review checklist is reproduced in `rfcs/README.md`. |

## Open Questions

None.
