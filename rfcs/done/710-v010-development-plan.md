# RFC 710 — v0.10 Development Plan

**Status.** Proposed  
**Tracks.** Governance

## Theme: Performance and Transport Security

| RFC | Feature | Rationale |
|-----|---------|-----------|
| 711 | Bulk SMTP parallelism | Completes v0.9 bulk send; RFC 701 deferred this as "v1.0 optimisation" |
| 712 | HTTP server TLS (HTTPS) | Transport security for direct-expose deployments |

## Scope boundaries

**In:**
- `[smtp].bulk_concurrency` config (default 5, 0 = unlimited)
- Two-phase bulk processing: sequential validate → parallel SMTP
- `[server].tls_cert` + `[server].tls_key` config
- TLS as optional Cargo feature (`--features tls`)
- OpenBSD: unveil cert/key before pledge; no new pledge promises needed

**Out:**
- Cert hot-reload / rotation
- mTLS / client certificate auth
- HTTP/2 (TLS only enables HTTPS/1.1 in v0.10)

## OpenBSD sequence for TLS

```
initial restrictions:
  unveil(config, "r")
  unveil(tls_cert, "r")   ← new if TLS configured
  unveil(tls_key,  "r")   ← new if TLS configured

load config
load TLS config (reads cert+key into memory)   ← before pledge
build AppState, bind socket (or skip for TLS)
runtime pledge: "stdio inet"   ← unchanged; no rpath needed after cert load
serve
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-710-01 | Bulk SMTP submissions run with bounded parallelism. |
| AC-710-02 | Result order in response always matches message index order. |
| AC-710-03 | `--features tls` build serves HTTPS. |
| AC-710-04 | Non-TLS build rejects `tls_cert` config at startup. |
| AC-710-05 | All tests pass in both default and tls builds. |
