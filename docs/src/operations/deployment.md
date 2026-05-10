# Deployment Guide

End-to-end guide for deploying `http-smtp-rele` as a managed system service
on OpenBSD and Linux. Covers installation, service setup, verification, and upgrades.

Before deploying, complete the [Security Checklist](./security-checklist.md).

---

## Prerequisites

### Runtime

- A running SMTP server reachable from the relay host
  (OpenSMTPD, Postfix, or any MTA accepting local submissions)
- Network access from the relay to the SMTP server port

### Build

```sh
# Rust 1.91+
rustc --version

# Standard build (in-memory status store only)
cargo build --release

# With SQLite persistent status store
cargo build --release --features sqlite

# With HTTPS support
cargo build --release --features tls
```

The release binary is at `target/release/http-smtp-rele`.

---

## OpenBSD Deployment

All deployment artefacts are in `contrib/openbsd/`.

### 1. Create the service user

```sh
doas useradd -r -s /sbin/nologin -d /var/empty _http_smtp_rele
```

### 2. Install the binary

```sh
doas install -m 555 -o root -g bin \
    target/release/http-smtp-rele \
    /usr/local/bin/http-smtp-rele
```

### 3. Install and configure

```sh
# Install example config
doas install -m 640 -o root -g _http_smtp_rele \
    examples/http-smtp-rele.toml \
    /etc/http-smtp-rele.toml

# Edit the config — minimum required:
#   [mail] default_from, allowed_recipient_domains
#   [[api_keys]] id, secret
doas vi /etc/http-smtp-rele.toml
```

### 4. Create runtime directories

```sh
# SQLite status store (if using --features sqlite)
doas install -d -o _http_smtp_rele -m 750 /var/db/http-smtp-rele

# Optional structured log directory
doas install -d -o _http_smtp_rele -m 750 /var/log/http-smtp-rele
```

### 5. Install the rc.d script

```sh
doas install -m 755 -o root -g bin \
    contrib/openbsd/rc.d/http_smtp_rele \
    /etc/rc.d/http_smtp_rele
```

### 6. Enable and start

```sh
doas rcctl enable http_smtp_rele
doas rcctl start  http_smtp_rele
doas rcctl check  http_smtp_rele
```

### Guided install (alternative)

The script at `contrib/openbsd/install.sh` performs steps 1–5 in one go:

```sh
doas sh contrib/openbsd/install.sh target/release/http-smtp-rele
```

### Managing the service

```sh
rcctl start   http_smtp_rele
rcctl stop    http_smtp_rele
rcctl restart http_smtp_rele
rcctl reload  http_smtp_rele    # SIGHUP — no restart needed for most changes
rcctl check   http_smtp_rele
```

### Overriding variables in `/etc/rc.conf.local`

```sh
# Custom config path
http_smtp_rele_config="/etc/myapp/relay.toml"

# Extra flags
http_smtp_rele_flags=""
```

---

## Linux Deployment (systemd)

All deployment artefacts are in `contrib/linux/`.

### 1. Install the binary

```sh
sudo install -m 755 target/release/http-smtp-rele \
    /usr/local/bin/http-smtp-rele
```

### 2. Install and configure

```sh
sudo install -m 640 examples/http-smtp-rele.toml \
    /etc/http-smtp-rele.toml

# Edit — minimum required:
#   [mail] default_from, allowed_recipient_domains
#   [[api_keys]] id, secret
sudo $EDITOR /etc/http-smtp-rele.toml
```

### 3. Install the systemd unit

```sh
sudo install -m 644 contrib/linux/http-smtp-rele.service \
    /etc/systemd/system/http-smtp-rele.service
sudo systemctl daemon-reload
```

### 4. Enable and start

```sh
sudo systemctl enable http-smtp-rele
sudo systemctl start  http-smtp-rele
sudo systemctl status http-smtp-rele
```

### Guided install (alternative)

```sh
sudo sh contrib/linux/install.sh target/release/http-smtp-rele
```

### Managing the service

```sh
systemctl start   http-smtp-rele
systemctl stop    http-smtp-rele
systemctl restart http-smtp-rele
systemctl reload  http-smtp-rele    # SIGHUP — no restart needed for most changes
systemctl status  http-smtp-rele
journalctl -u http-smtp-rele -f     # follow logs
```

### Overriding the config path

```sh
# Create a drop-in override
sudo systemctl edit http-smtp-rele
# In the editor, add:
[Service]
ExecStart=
ExecStart=/usr/local/bin/http-smtp-rele --config /etc/myapp/relay.toml
```

### Hardening notes

The unit file uses `DynamicUser=yes`, which means systemd creates a temporary
dedicated user automatically. The state directory (`/var/lib/http-smtp-rele`)
is created and owned by that user.

If you prefer a static user (e.g. for consistent UID in audit logs):

```sh
# Install sysusers config
sudo install -m 644 contrib/linux/http-smtp-rele.sysusers \
    /etc/sysusers.d/http-smtp-rele.conf
sudo systemd-sysusers

# Then remove DynamicUser=yes from the unit and set:
# User=http-smtp-rele
# Group=http-smtp-rele
```

---

## Post-install verification

Run these checks after starting the service on either platform.

### 1. Liveness

```sh
curl -s http://127.0.0.1:8080/healthz
# Expected: 200 OK, body: "ok"
```

### 2. SMTP readiness

```sh
curl -s http://127.0.0.1:8080/readyz
# Expected: 200 OK when SMTP server is reachable
# Expected: 503 when SMTP server is down
```

### 3. Test send

```sh
TOKEN="your-api-key-secret"
curl -s -X POST http://127.0.0.1:8080/v1/send \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"to":"you@example.com","subject":"Deploy test","body":"It works."}'
# Expected: 202 Accepted with request_id
```

### 4. Prometheus metrics

```sh
curl -s http://127.0.0.1:8080/metrics | grep rele_
# Expected: counters for requests, smtp submissions, etc.
```

---

## Upgrading

### Hot-swap (no downtime)

When only the binary changes (no config schema changes):

```sh
# Build new binary
cargo build --release

# Replace binary (service keeps running)
sudo install -m 555 target/release/http-smtp-rele \
    /usr/local/bin/http-smtp-rele

# Restart to load new binary
# OpenBSD:
doas rcctl restart http_smtp_rele

# Linux:
sudo systemctl restart http-smtp-rele
```

### Config-only change

Most `[status]`, `[rate_limit]`, `[logging]`, and `[mail]` fields take
effect on SIGHUP without a restart:

```sh
# OpenBSD:
doas rcctl reload http_smtp_rele

# Linux:
sudo systemctl reload http-smtp-rele
```

Fields that require restart: `[server]`, `[smtp]`, `[status].enabled`,
`[status].store`, `[status].db_path`, `[status].redis_url`.
See [Configuration Reference](../guides/configuration.md).

### Rolling back

```sh
# OpenBSD:
doas install -m 555 /path/to/previous/binary /usr/local/bin/http-smtp-rele
doas rcctl restart http_smtp_rele

# Linux:
sudo install -m 755 /path/to/previous/binary /usr/local/bin/http-smtp-rele
sudo systemctl restart http-smtp-rele
```
