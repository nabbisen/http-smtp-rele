# Getting Started

## Installation

### From source (requires Rust 1.75+)

```sh
git clone https://github.com/nabbisen/http-smtp-rele
cd http-smtp-rele
cargo build --release
# Binary at: target/release/http-smtp-rele
```

### From release archive

```sh
tar xzf http-smtp-rele-v0.1.0.tar.gz
cd http-smtp-rele-v0.1.0
# Binary included
```

---

## Minimum Configuration

Create `/etc/http-smtp-rele.toml`:

```toml
[mail]
default_from = "noreply@example.com"
allowed_recipient_domains = ["example.com"]

[[api_keys]]
id      = "myapp"
secret  = "replace-this-with-openssl-rand-base64-32"
enabled = true

[smtp]
host = "127.0.0.1"
port = 25
```

Generate a secret:
```sh
openssl rand -base64 32
```

---

## Starting the Relay

```sh
http-smtp-rele --config /etc/http-smtp-rele.toml
```

Default bind address: `http://127.0.0.1:8080`

Check liveness:
```sh
curl http://127.0.0.1:8080/healthz
# {"status":"ok","version":"0.1.0"}
```

Check SMTP readiness:
```sh
curl http://127.0.0.1:8080/readyz
# {"status":"ready","smtp":"ok"}
```

---

## Sending Mail

```sh
curl -X POST http://127.0.0.1:8080/v1/send \
  -H "Authorization: Bearer your-secret-here" \
  -H "Content-Type: application/json" \
  -d '{
    "to": "user@example.com",
    "subject": "Hello",
    "body": "This is a test message."
  }'
```

Successful response (`202 Accepted`):
```json
{"status": "accepted", "request_id": "..."}
```

---

## Next Steps

- [Configuration Reference](configuration.md) — all available options
- [API Reference](api.md) — complete endpoint documentation
- [Security](security.md) — securing your deployment
- [OpenBSD Deployment](openbsd.md) — production setup on OpenBSD

## Building with SQLite status store

The in-memory status store is included in all builds. For persistent status
storage that survives restarts, build with the `sqlite` feature:

```sh
# Standard build (in-memory status store only)
cargo build --release

# With SQLite status store
cargo build --release --features sqlite
```

The `sqlite` feature links the bundled SQLite C library (`libsqlite3`). No
system SQLite installation is required. The resulting binary is larger but
self-contained.

When using the SQLite store, create the database directory before starting:

```sh
# OpenBSD / Linux — adjust owner and path as needed
install -d -o _http_smtp_rele -m 750 /var/db/http-smtp-rele
```

Then configure `[status]` in your TOML:

```toml
[status]
store   = "sqlite"
db_path = "/var/db/http-smtp-rele/status.db"
```

The database file is created automatically on first run.
