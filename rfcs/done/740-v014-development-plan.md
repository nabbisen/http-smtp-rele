# RFC 740 — v0.14 Development Plan

**Status.** Proposed  
**Tracks.** Governance

## Theme: Deploy Automation

| RFC | Deliverable |
|-----|-------------|
| 741 | OpenBSD `rc.d` service script |
| 742 | Linux `systemd` unit file |
| 743 | Deploy automation docs (docs/src/operations/deployment.md) |

## Scope

All artefacts live in `contrib/` so they are never compiled by cargo
and do not affect the library or binary API.

```
contrib/
  openbsd/
    rc.d/http_smtp_rele   ← rc.d service script
    doas.conf.example     ← minimal doas privilege example
    install.sh            ← guided install helper
  linux/
    http-smtp-rele.service  ← systemd unit
    http-smtp-rele.sysusers ← systemd-sysusers user/group creation
    install.sh              ← guided install helper
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-740-01 | `contrib/openbsd/rc.d/http_smtp_rele` follows OpenBSD rc.d(8) conventions. |
| AC-740-02 | `contrib/linux/http-smtp-rele.service` follows systemd best practices. |
| AC-740-03 | `docs/src/operations/deployment.md` covers both platforms end-to-end. |
| AC-740-04 | `docs/src/SUMMARY.md` references the new page. |
| AC-740-05 | No code changes; all tests continue to pass. |
