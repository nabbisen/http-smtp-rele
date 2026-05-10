# RFC 827 — body_html Default-Off Policy

**Status.** Proposed  
**Tracks.** T1 — Security / T3 — Validation  
**Touches.** RFC 823 (covers the mechanism)

## Problem

The attack surface and policy rationale for HTML body are detailed in RFC 823
(feature gates). This RFC records the security policy documentation requirements
that accompany the `allow_html_body = false` default.

## Note

The mechanism for disabling `body_html` is defined in RFC 823
(`allow_html_body = false` config flag). This RFC records the security
policy rationale and documentation requirements.

## Security policy rationale

HTML email forwarded through a trusted relay can be used for:
- Phishing (spoofed sender combined with legitimate relay reputation)
- Tracking pixels (privacy violation for recipients)
- Unsafe external content (images, fonts, scripts in some clients)

`http-smtp-rele` does **not** sanitize HTML content. When `allow_html_body = true`,
the relay trusts that the calling application has validated the HTML.

## Required documentation

The configuration reference and security docs must state:

1. `allow_html_body = false` is the default and recommended setting.
2. When enabled, the relay is a pass-through: no HTML sanitization is performed.
3. HTML should only be enabled for trusted internal services, not general-purpose APIs.
4. Operators enabling HTML body should implement sanitization in the calling application.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-827-01 | RFC 823's `allow_html_body = false` is the default. |
| AC-827-02 | Security docs document the trust model for HTML body. |
| AC-827-03 | Example config has `allow_html_body = false` with explanatory comment. |
