# RFC 730 — v0.12 Development Plan

**Status.** Proposed  
**Tracks.** Governance

## Theme: Documentation and Security Checklist

| RFC | Feature |
|-----|---------|
| 731 | mdbook documentation structure — reader persona-based navigation |
| 732 | Security checklist — production deployment pre-flight |

## Reader persona mapping

| Persona | Entry points |
|---------|-------------|
| New user | Introduction → Quick Start → FAQ |
| Experienced user | API Reference → Configuration → Status Tracking |
| Operator / security | Security Checklist → OpenBSD Deployment |
| Contributor | Architecture → Testing → Contributing |

## docs/src layout

```
docs/
  book.toml
  src/
    SUMMARY.md
    introduction.md
    getting-started.md
    faq.md
    guides/
      api-reference.md
      configuration.md
      status-tracking.md
      bulk-sending.md
    operations/
      security-checklist.md
      openbsd.md
      reverse-proxy.md
    development/
      architecture.md
      testing.md
      contributing.md
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-730-01 | `docs/src/SUMMARY.md` builds valid mdbook navigation. |
| AC-730-02 | All SUMMARY entries resolve to existing files. |
| AC-730-03 | Security checklist covers all production threat vectors. |
| AC-730-04 | No code changes; all tests continue to pass. |
