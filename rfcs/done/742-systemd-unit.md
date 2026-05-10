# RFC 742 — Linux systemd Unit File

**Status.** Proposed  
**Tracks.** T6 — Operations  
**Touches.** `contrib/linux/http-smtp-rele.service`

## Summary

Provides a hardened systemd service unit so operators can manage
`http-smtp-rele` as a system service with `systemctl`.

## Hardening features

- `DynamicUser=yes` — ephemeral user, no persistent UID needed
- `PrivateTmp=yes`, `NoNewPrivileges=yes`
- `ProtectSystem=strict`, `ProtectHome=yes`
- `ReadWritePaths=/var/lib/http-smtp-rele`
- `CapabilityBoundingSet=` (no capabilities)
- `SystemCallFilter=@system-service`
- `Restart=on-failure`, `RestartSec=5s`
- `ExecReload=/bin/kill -HUP $MAINPID` for SIGHUP reload

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-742-01 | `systemctl start http-smtp-rele` starts the service. |
| AC-742-02 | `systemctl reload` sends SIGHUP. |
| AC-742-03 | `systemd-analyze security http-smtp-rele.service` scores well. |
| AC-742-04 | Unit file passes `systemd-analyze verify`. |
