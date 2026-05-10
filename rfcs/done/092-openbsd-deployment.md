# RFC 092 — OpenBSD rc.d and Deployment Layout

**Status.** Implemented  
**Tracks.** Platform  
**Touches.** `examples/openbsd-rc.d-http_smtp_rele`, `docs/openbsd.md`

## Summary

Define the `rc.d` script, file layout, user creation, and file permissions for a standard
OpenBSD deployment of `http-smtp-rele`.

## Motivation

A correct rc.d script and well-documented file layout reduce operator error during deployment.
Wrong permissions on the config file (which contains API key secrets) are a security risk
(NFR-OPS-003, AC-OBSD-004, NFR-SEC-002).

## Scope

- `_http_smtp_rele` system user creation.
- File placement: binary, config, rc.d script.
- File permissions for each file.
- `rcctl enable/start/stop/restart` workflow.
- Daemon restart on config change.

## Non-goals

- systemd unit (future; for Linux deployments).
- Log rotation (handled by `newsyslog(8)` on OpenBSD).

## Design

### System user

```sh
useradd -u 700 -d /var/empty -s /sbin/nologin \
        -c "http-smtp-rele daemon" _http_smtp_rele
```

UID 700 is in the range reserved for local system accounts. Adjust to avoid collision with
existing UIDs.

### File layout and permissions

| Path | Owner | Group | Mode | Purpose |
|------|-------|-------|------|---------|
| `/usr/local/bin/http-smtp-rele` | `root` | `bin` | `555` | Binary |
| `/etc/http-smtp-rele.toml` | `root` | `_http_smtp_rele` | `640` | Config (contains secrets) |
| `/etc/rc.d/http_smtp_rele` | `root` | `wheel` | `555` | rc.d script |

The config file is owned by root (write-protected from the daemon) and group-readable by
`_http_smtp_rele` (so the daemon can read it). This prevents the daemon from modifying its
own config.

### rc.d script (`examples/openbsd-rc.d-http_smtp_rele`)

```sh
#!/bin/ksh

daemon="/usr/local/bin/http-smtp-rele"
daemon_user="_http_smtp_rele"
daemon_flags=""

. /etc/rc.d/rc.subr

rc_reload=NO

rc_pre() {
    if [ ! -f /etc/http-smtp-rele.toml ]; then
        echo "Config file not found: /etc/http-smtp-rele.toml" >&2
        return 1
    fi
}

rc_cmd $1
```

### Workflow

```sh
# Initial setup
useradd -u 700 -d /var/empty -s /sbin/nologin -c "http-smtp-rele daemon" _http_smtp_rele
cp /path/to/http-smtp-rele /usr/local/bin/
chmod 555 /usr/local/bin/http-smtp-rele
install -o root -g _http_smtp_rele -m 640 /dev/null /etc/http-smtp-rele.toml
vi /etc/http-smtp-rele.toml
cp examples/openbsd-rc.d-http_smtp_rele /etc/rc.d/http_smtp_rele
chmod 555 /etc/rc.d/http_smtp_rele
rcctl enable http_smtp_rele
rcctl start http_smtp_rele
rcctl check http_smtp_rele

# Config change
vi /etc/http-smtp-rele.toml
rcctl restart http_smtp_rele
```

### Log viewing

```sh
# Real-time (syslog piping configured in rc.d or via newsyslog)
tail -f /var/log/daemon | grep http-smtp-rele

# Or if rc.d pipes stderr:
tail -f /var/log/http-smtp-rele.log
```

## Documentation Changes

- Create `examples/openbsd-rc.d-http_smtp_rele`.
- Create `docs/openbsd.md` with the full deployment guide.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-092-01 | `rcctl start http_smtp_rele` starts the daemon as `_http_smtp_rele`. |
| AC-092-02 | `rcctl stop http_smtp_rele` stops the daemon cleanly. |
| AC-092-03 | Config file permissions are `640`, owned root:`_http_smtp_rele`. |
| AC-092-04 | Binary permissions are `555`, owned root:bin. |
| AC-092-05 | `docs/openbsd.md` contains the full deployment procedure. |

## Open Questions

None.

