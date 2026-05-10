# FAQ

**Q: Can clients set the `From` address?**

No. The `From` address is always taken from `mail.default_from` in the config. Clients can
set a display name (`from_name`), but the email address itself is fixed. This is intentional:
it prevents impersonation and makes the relay's behaviour predictable.

---

**Q: Can I send to multiple recipients?**

Not in v0.1. The `to` field accepts a single address. Multi-recipient support is planned for
v0.2.

---

**Q: Can I send HTML mail?**

Not in v0.1. Only plain text body is supported. HTML support is planned for a future version.

---

**Q: What happens if the SMTP server is down?**

`POST /v1/send` returns `502 smtp_unavailable`. The message is not queued by the relay —
it is the caller's responsibility to retry. `GET /readyz` returns `503` when SMTP is
unreachable, allowing upstream load balancers to stop routing traffic to the relay.

---

**Q: Why does the relay listen on 127.0.0.1 by default?**

To prevent accidental exposure. Place a TLS-terminating reverse proxy (nginx, Caddy, relayd)
in front and configure it to proxy to `127.0.0.1:8080`. Never bind to `0.0.0.0` without a
firewall rule.

---

**Q: What does "rate limits reset on restart" mean?**

Rate limiting state is stored in memory. When the process restarts (upgrade, config change,
crash recovery), all token buckets start fresh with their full burst capacity. This means
up to `burst_size` requests are allowed immediately after a restart, even from clients that
had exceeded their limit.

---

**Q: Can I run multiple instances?**

Yes, but rate limits are per-instance. Instances do not share state. If you run multiple
instances behind a load balancer, each instance independently tracks its buckets. This
may allow more total requests than configured.

---

**Q: Does http-smtp-rele support SMTP AUTH?**

Not in v0.1. For localhost relay to OpenSMTPD, SMTP AUTH is not required. SMTP AUTH support
is planned for v0.2 to support non-localhost relay targets.

---

**Q: How do I rotate API keys without downtime?**

1. Add the new key to `[[api_keys]]` with `enabled = true`.
2. Restart the relay.
3. Update clients to use the new key.
4. Set `enabled = false` on the old key.
5. Restart again.

The relay continues serving requests with the old key during step 2–3.

---

**Q: Is http-smtp-rele suitable for high-volume bulk mail?**

No. It is designed for transactional mail at modest volume. The in-memory rate limiter and
synchronous SMTP submission are not designed for bulk throughput. Use a purpose-built bulk
mail system for high-volume use cases.
