# RFC 743 — Deployment Documentation

**Status.** Proposed  
**Tracks.** T6 — Documentation  
**Touches.** `docs/src/operations/deployment.md`, `docs/src/SUMMARY.md`

## Summary

End-to-end deployment guide covering both OpenBSD and Linux, referencing
the contrib/ artefacts and tying together the security checklist, configuration
reference, and reverse proxy setup pages.

## Sections

1. Prerequisites (Rust toolchain, SMTP server)
2. Building the binary (default / with features)
3. OpenBSD deployment (user, permissions, rc.d, verify)
4. Linux deployment (user, permissions, systemd, verify)
5. Post-install verification (health check, test send)
6. Upgrading (binary swap, SIGHUP vs restart)

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-743-01 | Guide is complete end-to-end for both platforms. |
| AC-743-02 | `docs/src/SUMMARY.md` references deployment.md. |
