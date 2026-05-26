# Security

## Security model

Yapper is designed around the principle that **the server should never be trusted with message content**.

| Threat | Mitigation |
|--------|-----------|
| Server compromise | All messages are E2EE — server stores only opaque ciphertext |
| Man-in-the-middle | Safety numbers (verify identity keys out-of-band) |
| Token theft | Short-lived JWTs (15 min) + HttpOnly refresh tokens |
| CSRF | Double-submit cookie (`X-CSRF-Token` required on all mutations) |
| Brute-force login | Per-IP rate limiting (governor) + login attempt tracking |
| SQL injection | sqlx parameterised queries — no string interpolation |
| XSS | SvelteKit escapes all template values by default |
| Clickjacking | `X-Frame-Options: DENY` via tower-http |
| Weak passwords | Argon2id hashing, minimum 8 characters enforced |
| Insecure uploads | Client-side AES-256-GCM encryption before R2 upload |
| Stale keys | OPKs are consumed atomically (`FOR UPDATE SKIP LOCKED`) |

## Authentication

- Passwords hashed with **Argon2id** (memory-hard, GPU-resistant)
- JWT access tokens: **RS256**, 15-minute expiry
- Refresh tokens: **HttpOnly, Secure, SameSite=None** cookies (cross-origin Tauri support)
- CSRF protection: every mutating request requires `X-CSRF-Token` matching the session cookie
- Nine routes are explicitly CSRF-exempt; all other mutating routes require the token:
  `/auth/login`, `/auth/register`, `/auth/verify-email`,
  `/auth/password-reset/request`, `/auth/password-reset/confirm`,
  `/auth/refresh`, `/auth/oauth/exchange`, `/premium/webhook`,
  `/support/webhooks/hubspot`

## E2EE

See [E2EE Implementation](E2EE-Implementation) for the full cryptographic design.

Key points:
- X3DH + double ratchet for DMs — forward secrecy per message
- Sender Keys for group channels — efficient N-party encryption
- Server stores only ciphertext — never plaintext, never encryption keys
- Safety numbers allow out-of-band identity verification

## Parental controls

Child accounts (under 18) have enforced restrictions that cannot be bypassed:

- Friend requests to a child must be approved by a parent before the E2EE session is established
- Server joins by a child require parent approval before membership is granted
- The parental approval workflow is enforced server-side — it cannot be bypassed from the client

COPPA compliance: `coppa_consent_verified_at` timestamp is recorded on child account creation.

## Secrets management

- **No secrets in the repository** — all secrets are in Fly.io secrets or GitHub Secrets
- JWT private key, OAuth client secrets, API tokens are all stored as encrypted Fly.io secrets
- `backend/secrets/` (JWT keys, Firebase service account) is gitignored
- `.env` files are gitignored

Public values that are safe to expose in the repo:
- Firebase VAPID public key (by design public)
- Sentry DSN (embedded in browser JS anyway)
- Cloudflare Account ID (not a secret per Cloudflare docs)

## Known advisories (ignored in CI)

| Advisory | Package | Reason ignored |
|----------|---------|---------------|
| RUSTSEC-2023-0071 | rsa (via sqlx-mysql/jsonwebtoken) | No patched `rsa` release exists. The sqlx-mysql path is unreachable because Yapper uses Postgres only; jsonwebtoken uses RSA for RS256 JWT signing/verification, so this remains a reviewed launch-risk item. |

## Responsible disclosure

If you discover a security vulnerability in Yapper, please **do not open a public GitHub issue**.

Instead, contact us privately:

1. Email: **security@yapperhq.com** (preferred)
2. GitHub: Use [GitHub Private Security Advisories](https://github.com/NikheelR97/Yapper/security/advisories/new)

Please include:
- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Your suggested fix (optional)

We aim to acknowledge reports within 48 hours and resolve critical issues within 7 days.

We do not currently offer a bug bounty program, but we will credit researchers who report valid issues in our release notes (if desired).
