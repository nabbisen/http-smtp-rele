# Deploying on OpenBSD

## Prerequisites

- OpenBSD 7.4 or later
- OpenSMTPD running (default on OpenBSD)
- `http-smtp-rele` binary built for OpenBSD

---

## System User

Create a dedicated unprivileged user:

```sh
useradd -u 700 -d /var/empty -s /sbin/nologin \
        -c "http-smtp-rele daemon" _http_smtp_rele
```

Choose a UID that does not conflict with existing system users (`id 700` is usually free;
verify with `getent passwd 700`).

---

## File Placement and Permissions

```sh
# Binary
cp http-smtp-rele /usr/local/bin/http-smtp-rele
chmod 555 /usr/local/bin/http-smtp-rele
chown root:bin /usr/local/bin/http-smtp-rele

# Config (contains secrets — group-readable only by the daemon user)
install -o root -g _http_smtp_rele -m 640 \
    /dev/null /etc/http-smtp-rele.toml
vi /etc/http-smtp-rele.toml

# rc.d script
cp examples/openbsd-rc.d-http_smtp_rele /etc/rc.d/http_smtp_rele
chmod 555 /etc/rc.d/http_smtp_rele
chown root:wheel /etc/rc.d/http_smtp_rele
```

| Path | Owner | Group | Mode | Purpose |
|------|-------|-------|------|---------|
| `/usr/local/bin/http-smtp-rele` | root | bin | 555 | Binary |
| `/etc/http-smtp-rele.toml` | root | `_http_smtp_rele` | 640 | Config (contains secrets) |
| `/etc/rc.d/http_smtp_rele` | root | wheel | 555 | rc.d script |

---

## rc.d Service Management

```sh
# Enable and start
rcctl enable http_smtp_rele
rcctl start  http_smtp_rele

# Verify
rcctl check  http_smtp_rele

# Stop
rcctl stop   http_smtp_rele

# Restart after config change
rcctl restart http_smtp_rele

# View flags
rcctl get    http_smtp_rele
```

The rc.d script (`examples/openbsd-rc.d-http_smtp_rele`) runs the daemon as
`_http_smtp_rele` and fails pre-flight if the config file is missing.

---

## OpenSMTPD Integration

The relay submits to `127.0.0.1:25` by default. This works with the default OpenSMTPD
configuration, which listens on loopback.

Verify OpenSMTPD is running and accepting local connections:

```sh
rcctl check smtpd
echo "QUIT" | nc -q1 127.0.0.1 25
# Expected: 220 hostname.example.com ESMTP OpenSMTPD
```

If SMTP is reachable, `/readyz` returns `{"status":"ready"}`:

```sh
curl http://127.0.0.1:8080/readyz
```

### Minimum smtpd.conf

The default `/etc/mail/smtpd.conf` on OpenBSD includes:
```
listen on lo0
```

No further changes are required for localhost relay. OpenSMTPD accepts unauthenticated
connections from localhost by default.

---

## pledge and unveil

After loading the config, `http-smtp-rele` applies OpenBSD sandboxing:

1. **`unveil(NULL, NULL)`** — locks the filesystem view. No file access is possible
   after this point (config is already loaded).

2. **`pledge("stdio inet")`** — restricts syscalls to:
   - `stdio`: read/write on existing descriptors (stderr for logs)
   - `inet`: TCP connections (incoming HTTP, outgoing SMTP)

If a pledge or unveil violation occurs, OpenBSD sends `SIGABRT` and the process terminates.
The kernel logs the violation to `/var/log/messages`.

**Why this pledge set works:** The config is read before sandboxing, so `rpath` is not
needed at runtime. SMTP uses a direct TCP connection (no `dns` needed when `smtp.host` is
an IP address).

> **Important:** Set `smtp.host = "127.0.0.1"` (not `"localhost"`). The `dns` promise is
> not included; hostname resolution is not available after pledge is applied.

---

## Verifying the Deployment

```sh
# Health check
curl http://127.0.0.1:8080/healthz

# SMTP readiness
curl http://127.0.0.1:8080/readyz

# Send a test mail (replace with your key and recipient)
API_KEY=your-secret TO=you@example.com sh examples/curl-send.sh
```

Check the log:
```sh
grep http_smtp_rele /var/log/daemon
```

---

## Upgrading

1. Build or download the new binary.
2. Copy to a temporary path: `cp http-smtp-rele /usr/local/bin/http-smtp-rele.new`
3. `rcctl stop http_smtp_rele`
4. `mv /usr/local/bin/http-smtp-rele.new /usr/local/bin/http-smtp-rele`
5. `rcctl start http_smtp_rele`
6. `rcctl check http_smtp_rele`

The stop/start sequence ensures the pledge/unveil state is re-applied from the new binary.
