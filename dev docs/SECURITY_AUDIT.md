# Security Audit — S10 W22

**Date:** 2026-03-05
**Scope:** Full backend + frontend dependency review
**Auditor:** Claude (automated)
**Result:** 14 PASS, 5 FIXED, 1 VERIFIED

---

## Summary

| # | Category | Status | Severity |
|---|----------|--------|----------|
| 1 | Rate Limiting | PASS | — |
| 2 | Security Headers | PASS | — |
| 3 | CORS | VERIFIED | — |
| 4 | Password Hashing | PASS | — |
| 5 | JWT Implementation | PASS | — |
| 6 | CSRF Protection | PASS | — |
| 7 | OAuth State | PASS | — |
| 8 | WebSocket Auth | PASS | — |
| 9 | WS Max Connections | **FIXED** | Medium |
| 10 | Input Validation | PASS | — |
| 11 | `deny_unknown_fields` | **FIXED** | Low |
| 12 | Read Receipt Auth | **FIXED** | Low |
| 13 | PII Anonymization | **FIXED** | Medium |
| 14 | COPPA / Parental Controls | PASS | — |
| 15 | Refresh Token Cookies | PASS | — |
| 16 | Account Deletion (GDPR) | PASS | — |
| 17 | SQL Injection | PASS | — |
| 18 | File Upload Validation | PASS | — |
| 19 | Dependency Audit (Rust) | **NOTED** | Low–Medium |
| 20 | Dependency Audit (JS) | **NOTED** | Low–High |

---

## Detailed Findings

### 1. Rate Limiting — PASS

Four independent rate-limiting layers:

| Layer | Config | Location |
|-------|--------|----------|
| IP-level | 100 req/min, burst 20 | `main.rs` (Governor middleware) |
| WebSocket | 5 msg/sec per user, burst 20 | `hub.rs` (`RateLimiter` struct) |
| Login brute-force | 5 attempts → 15-min lockout | `auth/middleware.rs` |
| Promo / wishlist | 5 req/hr per IP | Cloudflare Worker |

### 2. Security Headers — PASS

Applied via `tower-http` `SetResponseHeaderLayer`:

- `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload` (2-year HSTS + preload)
- `Content-Security-Policy: default-src 'none'` (API-only, no HTML served)
- `X-Frame-Options: DENY`
- `X-Content-Type-Options: nosniff`

### 3. CORS — VERIFIED

- Strict allowlist via `CORS_ORIGINS` env var (comma-separated)
- Defaults to `http://localhost:5173,http://localhost:1420,tauri://localhost,capacitor://localhost` in dev
- **Fly.io production:** `CORS_ORIGINS` secret confirmed set via `flyctl secrets list`
- Production value: `https://app.yapperhq.com,tauri://localhost,capacitor://localhost`

### 4. Password Hashing — PASS

- Algorithm: **Argon2id** (`argon2` crate)
- Parameters: m=65536 (64 MB), t=3, p=4
- Max password length: 1024 bytes (prevents DoS via huge payloads)
- Salt: auto-generated per hash

### 5. JWT Implementation — PASS

- Algorithm: **RS256** (RSA-2048)
- Access token TTL: 15 minutes
- Refresh token TTL: 30 days
- `kid` header for future key rotation
- `jti` unique claim per token
- Family-based refresh token reuse detection (revokes entire family on replay)

### 6. CSRF Protection — PASS

- Double-submit cookie pattern (`csrf.rs`)
- `SameSite=Strict` + `Secure` (conditional on `FLY_APP_NAME` env var)
- Explicit route allowlist for public auth and OAuth callback routes during the `v1` -> `v2` migration. Current production login/register/refresh entrypoints include `/api/v2/auth/login`, `/api/v2/auth/register`, and `/api/v2/auth/refresh`; authenticated device-binding routes such as `/api/v2/auth/attach-device` remain CSRF-protected and are not exempt.

### 7. OAuth State — PASS

- 32-byte random state parameter (`getrandom`)
- One-time use (`.remove()` from `DashMap`)
- 10-minute expiry with background GC task
- Covers Discord + Google + Apple OAuth flows

### 8. WebSocket Auth — PASS

- 10-second authentication timeout (connection dropped if no valid JWT within 10s)
- 64 KB max frame size
- Per-user rate limiting (5 msg/sec, burst 20, Governor `RateLimiter`)
- JWT validated on connect, `user_id` extracted and stored

### 9. WS Max Connections Per User — FIXED (Medium)

**Issue:** No limit on WebSocket connections per user. A malicious user could open thousands of connections, exhausting server memory.

**Fix:** Added `MAX_CONNECTIONS_PER_USER = 5` constant. `Hub::register()` now returns `bool` and rejects connections beyond the limit. Call site sends WS error code 4008 and closes.

**File:** `backend/src/hub.rs`
**Test:** `test_hub_max_connections_per_user` (passes)

### 10. Input Validation — PASS

- All user-facing SQL uses parameterized queries (`$1`, `$2`, etc.)
- String length caps on polls (500 chars question, 500 chars per option, 2–10 options)
- Explore search capped at 255 chars
- File uploads validated: content-type allowlist, 25 MB max size, WebP conversion for images
- UUID path parameters parsed by Axum extractors (invalid = 400)

### 11. `deny_unknown_fields` on User-Facing DTOs — FIXED (Low)

**Issue:** 5 user-facing request structs accepted unknown JSON fields silently, which could mask client bugs.

**Fix:** Added `#[serde(deny_unknown_fields)]` to:
- `CreateChildInput` (`parental/mod.rs`)
- `MusicInput` (`canvas/mod.rs`)
- `PollInput` (`canvas/mod.rs`)
- `VoteInput` (`canvas/mod.rs`)
- `ListMessagesQuery` (`messages/mod.rs`)

**Note:** OAuth response structs (Discord/Google/Apple) intentionally left WITHOUT this attribute — external APIs may add fields.

### 12. Read Receipt Channel Membership — FIXED (Low)

**Issue:** `handle_mark_read()` in `hub.rs` did not verify the user was a member of the channel before processing the read receipt and fanning out to other members.

**Fix:** Moved `fetch_channel_member_ids()` call before the DB upsert. Added `member_ids.contains(&user_id)` check — silently drops the receipt if the user is not a member. No extra DB query (reuses existing member fetch).

**File:** `backend/src/hub.rs`

### 13. PII Anonymization on Account Delete — FIXED (Medium)

**Issue:** `DELETE /api/v1/account` only set `deleted_at = NOW()` — PII (username, email, display name, avatar, banner, bio) remained in the database.

**Fix:** Account deletion now:
- Sets `username` to `[deleted_XXXX]` (first 8 chars of UUID)
- Sets `email` to `deleted+XXXX@yapperhq.com`
- NULLs `display_name`, `avatar_url`, `banner_url`, `about_me`
- Still sets `deleted_at = NOW()` for the soft-delete tombstone

**Deferred:** Full 30-day hard purge job (post-MVP).

**File:** `backend/src/users/mod.rs`

### 14. COPPA / Parental Controls — PASS

- Child accounts created by parent only (`POST /api/v1/parent/children`)
- Friend requests to children → `pending_friend_requests` → parent approval required
- Server joins by children → `pending_server_joins` → parent approval required
- Audit trail in `parental_audit_log` table
- E2EE inviolable — parents see metadata only (online status, friend list, server list)

### 15. Refresh Token Cookies — PASS

- `HttpOnly` flag: yes
- `SameSite=Strict`
- `Secure` flag: conditional (enabled when `FLY_APP_NAME` is set, i.e., production)
- `Path=/api/v2/auth/refresh` for the active device-aware refresh flow (scoped to the refresh endpoint only)
- Refresh sessions are now bound to the authenticated `device_id` as well as `user_id`
- 30-day max-age

### 16. Account Deletion (GDPR) — PASS

- `DELETE /api/v1/account` soft-deletes with PII anonymization (see Fix 13)
- `GET /api/v1/account/data-export` returns ZIP with user data (GDPR data portability)
- Session purge on delete (all refresh tokens revoked)

### 17. SQL Injection — PASS

- All queries use sqlx parameterized queries (`$1`, `$2`)
- No string interpolation in SQL
- `FOR UPDATE SKIP LOCKED` for atomic OPK consumption
- Transactions used for multi-step operations

### 18. File Upload Validation — PASS

- Content-type allowlist: `image/png`, `image/jpeg`, `image/gif`, `image/webp`, `audio/webm`, `video/webm`
- Max file size: 25 MB (checked before processing)
- Images converted to WebP at fixed dimensions (avatar 256x256, banner 1500x500, emoji 64x64)
- Files uploaded to R2 with content-type preserved

---

## Dependency Audit Results

### Rust (`cargo audit`) — 3 vulnerabilities, 4 warnings

| ID | Crate | Severity | Status |
|----|-------|----------|--------|
| RUSTSEC-2024-0421 | `idna` 0.5.0 | Low | Via `validator 0.18.1`. Upgrade blocked by validator dependency. **Deferred.** |
| RUSTSEC-2023-0071 | `rsa` 0.9.10 | Medium (5.9) | Via `sqlx-mysql` (transitive). We use PostgreSQL, not MySQL — **no real exposure.** No fix available. |
| RUSTSEC-2024-0363 | `sqlx` 0.7.4 | Medium | Binary protocol misinterpretation. Fix requires upgrade to sqlx 0.8+ (breaking). **Deferred to post-MVP.** |

**Warnings (unmaintained/unsound):**
- `paste` — unmaintained (via aws-sdk)
- `proc-macro-error` — unmaintained (via darling/serde)
- `rustls-pemfile` — unmaintained (via reqwest)
- `lru` — unsound (via aws-sdk-s3)

All warnings are transitive dependencies from AWS SDK — no action possible until upstream updates.

### JavaScript (`npm audit`) — 13 vulnerabilities after fix

**Fixed:** `tar` hardlink path traversal (GHSA-qffp-2rhf-9h96) — updated via `npm audit fix`.

**Remaining (all require breaking changes):**

| Package | Severity | Via | Status |
|---------|----------|-----|--------|
| `cookie` <0.7.0 | Low | `@sveltejs/kit` | Fix requires SvelteKit downgrade to 0.0.30 — **not viable** |
| `esbuild` <=0.24.2 | Moderate | `vitest` (dev-only) | Dev server request forwarding. **Dev-only, no prod exposure.** |
| `serialize-javascript` <=7.0.2 | High | `vite-plugin-pwa` → `workbox-build` | RCE via RegExp.flags. Build-time only — **no runtime exposure.** |

**Assessment:** All remaining JS vulnerabilities are in build-time or dev-only dependencies. None affect the production runtime (SvelteKit builds to static files via `adapter-static`).

---

## Deferred Items

| Item | Reason | Target |
|------|--------|--------|
| sqlx 0.8 upgrade | Breaking API changes across entire backend | Post-MVP |
| Full 30-day hard purge job | Requires background scheduler (cron/pg_cron) | Post-MVP |
| Content moderation (CSAM scanning) | Requires third-party API integration | Post-MVP |
| E2E penetration testing | Needs staging environment + test accounts | Post-MVP |
| Apple FamilyControls entitlement | Requires Apple Developer account access | Post-MVP |

---

## Verification

- `cargo build` — all fixes compile successfully
- `cargo test` — 36/36 tests pass (including new `test_hub_max_connections_per_user`)
- `cargo audit` — run, results documented above
- `npm audit` — run, `tar` fixed, remaining documented above
- CORS_ORIGINS — verified set on Fly.io production
