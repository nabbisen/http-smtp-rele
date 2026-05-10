# http-smtp-rele

[![crates.io](https://img.shields.io/crates/v/http-smtp-rele?label=rust)](https://crates.io/crates/http-smtp-rele)
[![License](https://img.shields.io/github/license/nabbisen/http-smtp-rele)](https://github.com/nabbisen/http-smtp-rele/blob/main/LICENSE)
[![Documentation](https://docs.rs/http-smtp-rele/badge.svg?version=latest)](https://docs.rs/http-smtp-rele)
[![Dependency Status](https://deps.rs/crate/http-smtp-rele/latest/status.svg)](https://deps.rs/crate/http-smtp-rele)

**A minimal, secure HTTP-to-SMTP submission relay (relé) written in Rust.**

---

## Overview

`http-smtp-rele` accepts JSON mail requests over HTTP, validates and sanitizes them, and
relays them to a local SMTP server (such as OpenSMTPD). It acts as a controlled gateway
between application code and the mail system.

---

## Why / When

Use `http-smtp-rele` when your application needs to send transactional mail and you want:

- **A single, auditable submission path** — all outgoing mail passes through one choke point
  with structured logs.
- **Open relay prevention** — `From` is always config-controlled, recipient domains are
  allowlisted, and unknown JSON fields are rejected.
- **Minimal attack surface** — on OpenBSD, `pledge("stdio inet")` and `unveil` restrict
  the process to the minimum required syscalls and filesystem access.
- **Simple integration** — any HTTP client that can POST JSON can send mail; no SMTP library
  needed in application code.

**Not for:** high-volume bulk mail, direct internet delivery (use an MTA with a smart host),
or multi-tenant SaaS (rate limits are in-memory and reset on restart).

---

## Quick Start

### 1. Build

```sh
cargo build --release
```

Or download a release archive.

### 2. Configure

```sh
cp examples/http-smtp-rele.toml /etc/http-smtp-rele.toml
```

Edit the config file — minimum required fields:

```toml
[mail]
default_from = "noreply@yourdomain.com"
allowed_recipient_domains = ["yourdomain.com"]

[[api_keys]]
id     = "myapp"
secret = "generate-with-openssl-rand-base64-32"
enabled = true
```

### 3. Start

```sh
http-smtp-rele --config /etc/http-smtp-rele.toml
```

### 4. Send a test mail

```sh
curl -X POST http://127.0.0.1:8080/v1/send \
  -H "Authorization: Bearer your-secret-here" \
  -H "Content-Type: application/json" \
  -d '{"to":"you@yourdomain.com","subject":"Test","body":"Hello from http-smtp-rele"}'
```

A successful response returns `202 Accepted`:

```json
{"status": "accepted", "request_id": "..."}
```

---

## Design Notes

- **No raw header concatenation** — all mail is built through `lettre`'s typed API.
- **Constant-time auth** — all API keys are compared in constant time with `subtle::ConstantTimeEq`; the auth loop never short-circuits.
- **Reject, never strip** — CR/LF in header-bound fields returns 400; it is never silently removed.
- **Secrets never logged** — `SecretString` has a redacted `Debug` implementation; the request body is always excluded from tracing spans.

> **Security:** Read [docs/security.md](docs/security.md) before exposing this relay to any network.

---

## For more detail

See the [full documentation](docs/README.md):

- [Getting started](docs/getting-started.md)
- [Configuration reference](docs/configuration.md)
- [API reference](docs/api.md)
- [Security](docs/security.md)
- [OpenBSD deployment](docs/openbsd.md)
