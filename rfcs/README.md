# http-smtp-rele RFCs

This directory contains all design RFCs for `http-smtp-rele`.
The folder location is the source of truth for each RFC's state.

Run `scripts/check-rfcs.sh` to verify structural integrity.

---

## Proposed

No RFCs currently open. All v0.3 items are in `done/`.

*(Next milestone RFCs will appear here.)*

---

### v0.3 Planning (archived)

| ID  | Title |
|-----|-------|
| 300 | [v0.3 Development Plan](./done/300-v03-development-plan.md) |
| 301 | [SMTP AUTH](./done/301-smtp-auth.md) |
| 302 | [Multi-Recipient to](./done/302-multi-recipient.md) |
| 303 | [W3C Forwarded Header](./done/303-forwarded-header.md) |
| 304 | [Sendmail Pipe Mode](./done/304-sendmail-pipe-mode.md) |
| 305 | [SIGHUP Config Reload](./done/305-sighup-reload.md) |

### v0.4 Features

| ID  | Title |
|-----|-------|
| 400 | [v0.4 Development Plan](./done/400-v04-development-plan.md) |
| 401 | [Prometheus /metrics Endpoint](./done/401-prometheus-metrics.md) |
| 402 | [SMTP STARTTLS and TLS](./done/402-smtp-starttls.md) |
| 403 | [HTML Body](./done/403-html-body.md) |
| 404 | [cc Recipients](./done/404-cc-recipients.md) |

### v0.5 Features

| ID  | Title |
|-----|-------|
| 500 | [v0.5 Development Plan](./done/500-v05-development-plan.md) |
| 501 | [Cargo Workspace Split](./done/501-workspace-split.md) |
| 502 | [Attachment Support](./done/502-attachments.md) |
| 503 | [reply\_to Array](./done/503-reply-to-array.md) |
| 504 | [Prometheus: Full Instrumentation](./done/504-prometheus-full.md) |

### v0.6 Features — Submission Status Tracking

| ID  | Title |
|-----|-------|
| 036 | [Submission Status API](./done/036-submission-status-api.md) |
| 086 | [Submission Status Store Abstraction](./done/086-submission-status-store-abstraction.md) |
| 087 | [In-memory Submission Status Store](./done/087-in-memory-status-store.md) |
| 088 | [Persistent Status Store Options](./done/088-persistent-status-store-options.md) |
| 089 | [Submission Status Store Metrics](./done/089-submission-status-metrics.md) |
| 106 | [Submission Status API Integration Tests](./done/106-submission-status-integration-tests.md) |

### v0.7 Features — Observability and Operations

| ID  | Title |
|-----|-------|
| 600 | [v0.7 Development Plan](./done/600-v07-development-plan.md) |
| 601 | [Status Store Prometheus Metrics](./done/601-status-store-metrics.md) |
| 602 | [GET /v1/keys/self — Authenticated Key Info](./done/602-key-info-endpoint.md) |
| 603 | [Per-Key mask\_recipient Override](./done/603-per-key-mask-recipient.md) |

### v0.8 — SQLite Persistent Status Store

| ID  | Title |
|-----|-------|
| 088 | [Persistent Status Store Options](./done/088-persistent-status-store-options.md) |

### v0.9 — Bulk Submission

| ID  | Title |
|-----|-------|
| 700 | [v0.9 Development Plan](./done/700-v09-development-plan.md) |
| 701 | [POST /v1/send-bulk API](./done/701-bulk-send-api.md) |
| 702 | [Bulk Submission Rate Limiting](./done/702-bulk-send-rate-limiting.md) |
| 703 | [Bulk Send Integration Tests](./done/703-bulk-send-tests.md) |

### T0 — Governance

| ID  | Title | Milestone |
|-----|-------|-----------|
| 001 | [RFC Directory Structure and Lifecycle Adoption](./done/001-rfc-governance.md) | M0 |
| 002 | [RFC Template and Review Checklist](./done/002-rfc-template.md) | M0 |
| 003 | [RFC Index and Integrity Check](./done/003-rfc-integrity-check.md) | M0 |
| 004 | [Project Quality Gates](./done/004-project-quality-gates.md) | M0 |

### T1 — Foundation

| ID  | Title | Milestone |
|-----|-------|-----------|
| 010 | [Runtime Architecture and Crate Structure](./done/010-runtime-architecture.md) | M1 |
| 011 | [Application State and Request Context](./done/011-application-state-request-context.md) | M1 |
| 012 | [Error Model and HTTP Response Mapping](./done/012-error-model.md) | M1 |
| 013 | [Logging Foundation](./done/013-logging-foundation.md) | M1 |
| 014 | [Graceful Startup and Shutdown](./done/014-graceful-startup-shutdown.md) | M1 |
| 020 | [TOML Configuration Schema](./done/020-configuration-schema.md) | M2 |
| 021 | [Configuration Loading and Fail-Fast Validation](./done/021-config-loading-validation.md) | M2 |
| 022 | [Secret Handling and Redaction](./done/022-secret-handling-redaction.md) | M2 |
| 023 | [Mail Policy Configuration](./done/023-mail-policy-configuration.md) | M2 |
| 024 | [Server and Security Configuration](./done/024-server-security-configuration.md) | M2 |
| 025 | [SMTP Configuration](./done/025-smtp-configuration.md) | M2 |

### T2 — HTTP API

| ID  | Title | Milestone |
|-----|-------|-----------|
| 030 | [HTTP API Surface and Versioning](./done/030-http-api-surface.md) | M3 |
| 031 | [Request and Response JSON Contract](./done/031-json-request-response-contract.md) | M3 |
| 032 | [Error Response Contract](./done/032-error-response-contract.md) | M3 |
| 033 | [Content-Type and Body Handling](./done/033-content-type-body-handling.md) | M3 |
| 034 | [Health and Readiness Endpoints](./done/034-health-readiness-endpoints.md) | M3 |
| 035 | [Request ID Response Policy](./done/035-request-id-policy.md) | M3 |

### T3 — Security Gate

| ID  | Title | Milestone |
|-----|-------|-----------|
| 040 | [API Key Authentication Model](./done/040-api-key-authentication.md) | M4 |
| 041 | [Source IP and Trusted Proxy Handling](./done/041-source-ip-trusted-proxy.md) | M4 |
| 042 | [API Key Policy and Per-Key Permissions](./done/042-api-key-policy-permissions.md) | M4 |
| 043 | [Constant-Time Comparison and Timing Safety](./done/043-constant-time-comparison.md) | M4 |
| 044 | [Authentication Failure Behavior](./done/044-authentication-failure-behavior.md) | M4 |
| 050 | [Strict Request Validation](./done/050-strict-request-validation.md) | M5 |
| 051 | [Header Injection Prevention](./done/051-header-injection-prevention.md) | M5 |
| 052 | [Recipient Address Validation](./done/052-recipient-address-validation.md) | M5 |
| 053 | [Body and Subject Limits](./done/053-body-subject-limits.md) | M5 |

### T4 — Mail Relay

| ID  | Title | Milestone |
|-----|-------|-----------|
| 060 | [Safe Plain Text Mail Construction](./done/060-mail-construction.md) | M6 |
| 061 | [SMTP Relay Transport](./done/061-smtp-relay-transport.md) | M6 |
| 062 | [SMTP Error Mapping and Timeout](./done/062-smtp-error-mapping.md) | M6 |
| 063 | [Readiness Check Behavior](./done/063-readiness-check.md) | M6 |
| 064 | [Sendmail Pipe Mode Deferral](./done/064-sendmail-pipe-deferral.md) | M6 |

### T5 — Abuse / Audit

| ID  | Title | Milestone |
|-----|-------|-----------|
| 070 | [Rate Limit Model](./done/070-rate-limit-model.md) | M7 |
| 071 | [Token Bucket Implementation](./done/071-token-bucket.md) | M7 |
| 072 | [Rate Limit Pipeline Ordering](./done/072-rate-limit-ordering.md) | M7 |
| 073 | [Rate Limited Response and Retry-After](./done/073-rate-limited-response.md) | M7 |
| 074 | [Abuse Prevention Policy](./done/074-abuse-prevention-policy.md) | M7 |
| 080 | [Tracing and Request Span Model](./done/080-tracing-request-span.md) | M8 |
| 081 | [Audit Event Taxonomy](./done/081-audit-event-taxonomy.md) | M8 |
| 082 | [Secret and Body Log Redaction](./done/082-secret-body-redaction.md) | M8 |
| 083 | [Recipient Masking Policy](./done/083-recipient-masking.md) | M8 |
| 084 | [Log Format Configuration](./done/084-log-format-configuration.md) | M8 |
| 085 | [Failure Observability](./done/085-failure-observability.md) | M8 |



---

## Implemented (v0.2 — in progress)

### v0.2 Planning and Test Infrastructure

| ID  | Title |
|-----|-------|
| 100 | [Integration Test Harness](./done/100-integration-test-harness.md) |
| 101 | [SMTP Stub Server](./done/101-smtp-stub-server.md) |
| 102 | [Security Regression Test Suite](./done/102-security-regression-tests.md) |
| 103 | [E2E Test Scenarios](./done/103-e2e-test-scenarios.md) |

### v0.2 Features

| ID  | Title |
|-----|-------|
| 200 | [v0.2 Development Plan](./done/200-v02-development-plan.md) |
| 201 | [Per-Tier Burst Configuration](./done/201-per-tier-burst-config.md) |
| 202 | [Default Per-Key Rate in \[rate_limit\]](./done/202-default-per-key-rate.md) |
| 203 | [Per-Key Burst Override](./done/203-per-key-burst-override.md) |
| 204 | [Per-Address Recipient Allowlist](./done/204-per-address-allowlist.md) |
| 205 | [Server Concurrency Limit](./done/205-concurrency-limit.md) |
| 206 | [IP Bucket LRU Eviction](./done/206-ip-bucket-lru-eviction.md) |

### v0.3 Features

| ID  | Title |
|-----|-------|
| 300 | [v0.3 Development Plan](./done/300-v03-development-plan.md) |
| 301 | [SMTP AUTH](./done/301-smtp-auth.md) |
| 302 | [Multi-Recipient to](./done/302-multi-recipient.md) |
| 303 | [W3C Forwarded Header](./done/303-forwarded-header.md) |
| 304 | [Sendmail Pipe Mode](./done/304-sendmail-pipe-mode.md) |
| 305 | [SIGHUP Config Reload](./done/305-sighup-reload.md) |

### v0.3 Features

| ID  | Title |
|-----|-------|
| 300 | [v0.3 Development Plan](./done/300-v03-development-plan.md) |
| 301 | [SMTP AUTH](./done/301-smtp-auth.md) |
| 302 | [Multi-Recipient to](./done/302-multi-recipient.md) |
| 303 | [W3C Forwarded Header](./done/303-forwarded-header.md) |
| 304 | [Sendmail Pipe Mode](./done/304-sendmail-pipe-mode.md) |
| 305 | [SIGHUP Config Reload](./done/305-sighup-reload.md) |
| 200 | [v0.2 Development Plan](./done/200-v02-development-plan.md) |

---

## Proposed — v0.2 Feature RFCs

| ID  | Title |
|-----|-------|
| 201 | [Per-Tier Burst Configuration](./done/201-per-tier-burst-config.md) |
| 202 | [Default Per-Key Rate Limit in `[rate_limit]`](./done/202-default-per-key-rate.md) |
| 203 | [Per-Key Burst Override](./done/203-per-key-burst-override.md) |
| 204 | [Per-Address Recipient Allowlist](./done/204-per-address-allowlist.md) |
| 205 | [Concurrency Limit](./done/205-concurrency-limit.md) |
| 206 | [IP Bucket LRU Eviction](./done/206-ip-bucket-lru-eviction.md) |

---

## Implemented (v0.1.0)

RFCs whose work shipped in v0.1.0. Moved from `proposed/` upon implementation.

### T0 — Governance

| ID  | Title |
|-----|-------|
| 001 | [RFC Directory Structure and Lifecycle Adoption](./done/001-rfc-governance.md) |
| 002 | [RFC Template and Review Checklist](./done/002-rfc-template.md) |
| 003 | [RFC Index and Integrity Check](./done/003-rfc-integrity-check.md) |
| 004 | [Project Quality Gates](./done/004-project-quality-gates.md) |

### T1 — Foundation

| ID  | Title |
|-----|-------|
| 010 | [Runtime Architecture and Crate Structure](./done/010-runtime-architecture.md) |
| 011 | [Application State and Request Context](./done/011-application-state-request-context.md) |
| 012 | [Error Model and HTTP Response Mapping](./done/012-error-model.md) |
| 013 | [Logging Foundation](./done/013-logging-foundation.md) |
| 014 | [Graceful Startup and Shutdown](./done/014-graceful-startup-shutdown.md) |
| 020 | [TOML Configuration Schema](./done/020-configuration-schema.md) |
| 021 | [Configuration Loading and Fail-Fast Validation](./done/021-config-loading-validation.md) |
| 022 | [Secret Handling and Redaction](./done/022-secret-handling-redaction.md) |
| 023 | [Mail Policy Configuration](./done/023-mail-policy-configuration.md) |
| 024 | [Server and Security Configuration](./done/024-server-security-configuration.md) |
| 025 | [SMTP Configuration](./done/025-smtp-configuration.md) |

### T2 — HTTP API

| ID  | Title |
|-----|-------|
| 030 | [HTTP API Surface and Versioning](./done/030-http-api-surface.md) |
| 031 | [Request and Response JSON Contract](./done/031-json-request-response-contract.md) |
| 032 | [Error Response Contract](./done/032-error-response-contract.md) |
| 033 | [Content-Type and Body Handling](./done/033-content-type-body-handling.md) |
| 034 | [Health and Readiness Endpoints](./done/034-health-readiness-endpoints.md) |
| 035 | [Request ID Response Policy](./done/035-request-id-policy.md) |

### T3 — Security Gate

| ID  | Title |
|-----|-------|
| 040 | [API Key Authentication Model](./done/040-api-key-authentication.md) |
| 041 | [Source IP and Trusted Proxy Handling](./done/041-source-ip-trusted-proxy.md) |
| 042 | [API Key Policy and Per-Key Permissions](./done/042-api-key-policy-permissions.md) |
| 043 | [Constant-Time Comparison and Timing Safety](./done/043-constant-time-comparison.md) |
| 044 | [Authentication Failure Behavior](./done/044-authentication-failure-behavior.md) |
| 050 | [Strict Request Validation](./done/050-strict-request-validation.md) |
| 051 | [Header Injection Prevention](./done/051-header-injection-prevention.md) |
| 052 | [Recipient Address Validation](./done/052-recipient-address-validation.md) |
| 053 | [Body and Subject Limits](./done/053-body-subject-limits.md) |

### T4 — Mail Relay

| ID  | Title |
|-----|-------|
| 060 | [Safe Plain Text Mail Construction](./done/060-mail-construction.md) |
| 061 | [SMTP Relay Transport](./done/061-smtp-relay-transport.md) |
| 062 | [SMTP Error Mapping and Timeout](./done/062-smtp-error-mapping.md) |
| 063 | [Readiness Check Behavior](./done/063-readiness-check.md) |
| 064 | [Sendmail Pipe Mode Deferral](./done/064-sendmail-pipe-deferral.md) |

### T5 — Abuse / Audit

| ID  | Title |
|-----|-------|
| 070 | [Rate Limit Model](./done/070-rate-limit-model.md) |
| 071 | [Token Bucket Implementation](./done/071-token-bucket.md) |
| 072 | [Rate Limit Pipeline Ordering](./done/072-rate-limit-ordering.md) |
| 073 | [Rate Limited Response and Retry-After](./done/073-rate-limited-response.md) |
| 074 | [Abuse Prevention Policy](./done/074-abuse-prevention-policy.md) |
| 080 | [Tracing and Request Span Model](./done/080-tracing-request-span.md) |
| 081 | [Audit Event Taxonomy](./done/081-audit-event-taxonomy.md) |
| 082 | [Secret and Body Log Redaction](./done/082-secret-body-redaction.md) |
| 083 | [Recipient Masking Policy](./done/083-recipient-masking.md) |
| 084 | [Log Format Configuration](./done/084-log-format-configuration.md) |
| 085 | [Failure Observability](./done/085-failure-observability.md) |

### T6 — Platform / Release

| ID  | Title |
|-----|-------|
| 090 | [OpenBSD Runtime Hardening](./done/090-openbsd-runtime-hardening.md) |
| 091 | [pledge and unveil Strategy](./done/091-pledge-unveil-strategy.md) |
| 092 | [OpenBSD rc.d and Deployment Layout](./done/092-openbsd-deployment.md) |
| 093 | [OpenSMTPD Localhost Relay Integration](./done/093-opensmtpd-integration.md) |
| 110 | [Documentation Structure](./done/110-documentation-structure.md) |
| 111 | [README and Quick Start](./done/111-readme-quick-start.md) |
| 112 | [API Documentation](./done/112-api-documentation.md) |
| 113 | [Configuration Documentation](./done/113-configuration-documentation.md) |
| 114 | [Security Documentation](./done/114-security-documentation.md) |
| 115 | [OpenBSD Deployment Documentation](./done/115-openbsd-deployment-documentation.md) |
| 116 | [Testing Documentation](./done/116-testing-documentation.md) |
| 117 | [Release Artifact Policy](./done/117-release-artifact-policy.md) |
| 120 | [MVP Release Criteria](./done/120-mvp-release-criteria.md) |

### T6 — Testing (v0.2)

| ID  | Title |
|-----|-------|
| 100 | [Integration Test Harness](./done/100-integration-test-harness.md) |
| 101 | [SMTP Stub Server](./done/101-smtp-stub-server.md) |
| 102 | [Security Regression Test Suite](./done/102-security-regression-tests.md) |
| 103 | [E2E Test Scenarios](./done/103-e2e-test-scenarios.md) |

---

## Archive

Withdrawn or superseded RFCs. Never deleted.

| ID  | Title | Reason |
|-----|-------|--------|
| — | — | — |

---

## RFC lifecycle

See [000-rfc-lifecycle-policy](../000-rfc-lifecycle-policy.md) for the full governance policy.

State transitions:
- `proposed/ → done/` when implementation ships.
- `proposed/ → archive/` when withdrawn.

Move the file and update this README in the same commit.

## Review checklist

Before acting on a `proposed/` RFC, verify:

```
[ ] Summary accurately reflects the RFC.
[ ] Motivation references at least one requirement.
[ ] Scope is a concrete list.
[ ] Non-goals explicitly state exclusions.
[ ] Design has enough detail to implement without ambiguity.
[ ] Test Plan has at least one test per Acceptance Criterion.
[ ] Security Considerations are present.
[ ] Acceptance Criteria are numbered and verifiable.
[ ] File is named NNN-slug.md and listed here.
[ ] Status field is "Proposed".
```
