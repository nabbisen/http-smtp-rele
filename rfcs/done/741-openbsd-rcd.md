# RFC 741 — OpenBSD rc.d Service Script

**Status.** Proposed  
**Tracks.** T6 — Operations  
**Touches.** `contrib/openbsd/rc.d/http_smtp_rele`

## Summary

Provides a standard OpenBSD rc.d(8) service script so operators can manage
`http-smtp-rele` as a system service with `rcctl`.

## Script requirements

- Dedicated service user `_http_smtp_rele` (non-login, nologin shell)
- Default config path `/etc/http-smtp-rele.toml`
- PID file at `/var/run/http-smtp-rele.pid`
- Daemon started with `daemon_execdir` and proper environment
- `rcctl reload` sends SIGHUP for config reload (RFC 721)

## File ownership and permissions

```sh
install -d -o _http_smtp_rele -m 750 /var/db/http-smtp-rele  # SQLite dir
install -d -o _http_smtp_rele -m 750 /var/log/http-smtp-rele # log dir
chown root:_http_smtp_rele /etc/http-smtp-rele.toml
chmod 640 /etc/http-smtp-rele.toml
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-741-01 | `rcctl start http_smtp_rele` starts the daemon. |
| AC-741-02 | `rcctl stop` gracefully shuts down. |
| AC-741-03 | `rcctl reload` sends SIGHUP. |
| AC-741-04 | Script uses `/etc/rc.d/daemon` base class. |
