# Yapper Stabilization Audit Report

**Auditor:** Senior Full-Stack Systems Architect and Security Auditor
**Codebase state:** S0-S16 complete, HANDOVER rev 6 (2026-04-06), 30-day stabilization phase
**Prior baseline:** 0 Critical, 0 High, 2 Medium, 4 Low (post-S14, see `SECURITY_AUDIT.md`)
**Date:** 2026-04-16

---

## Production Automation Verdict

**ALL DOCUMENTED GATES SATISFIED — UNPAUSE REQUIRES OWNER APPROVAL**

Original verdict (2026-04-16): "NOT SAFE TO UNPAUSE" — gates undefined (MED-005), HANDOVER crypto-library wording inconsistent (MED-006).

Re-verification (2026-05-27, against `main @ dbe09fb`): **all 10 documented release gates PASS** (see [Re-verification](#re-verification-2026-05-27) below). MED-005 and MED-006 are resolved.

Re-enabling `flyctl deploy` automation is not blocked by documentation any longer, but it remains a state-changing infrastructure action that requires:
- Explicit owner approval.
- Required reviewers configured on the GitHub `production` environment.
- Uncommenting the `deploy-backend` / `deploy-frontend` jobs in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) (lines 209-255).

The doc gates do not, by themselves, authorize the operational change.

---

## Release Gate Checklist

This checklist defines the release gates that must be satisfied before `flyctl deploy` automation resumes. Its creation directly addresses finding MED-005.

| # | Gate | Status | Evidence | Blocking? |
|---|------|--------|----------|-----------|
| 1 | All CI checks pass on `main` (backend, frontend, marketing, security-scans) | PASS | CI workflow has no `needs:` chain; all jobs run in parallel. Deploy jobs commented out (ci.yml:202-255) | Yes |
| 2 | `cargo audit` clean (1 known ignore with rationale) | PASS | [`backend/.cargo/audit.toml`](../backend/.cargo/audit.toml) — RUSTSEC-2023-0071 documented with rationale, review date 2026-05-26 | Yes |
| 3 | `cargo clippy -D warnings` clean | PASS | ci.yml:102 enforces on every push/PR | Yes |
| 4 | `npm audit --omit=dev` clean (frontend + marketing) | PASS | security-scans.yml:54-60 | Yes |
| 5 | Gitleaks full history scan clean | PASS | security-scans.yml:62-66 | Yes |
| 6 | Trivy filesystem + image scan (HIGH/CRITICAL) clean | PASS | security-scans.yml:94-116, trivy-action@v0.35.0 | Yes |
| 7 | E2E smoke tests pass (`--grep "@smoke"`) | PASS | e2e-pr-smoke.yml:162, workers=4 | Yes |
| 8 | No open Critical or High audit findings | PASS | This audit: 0 Critical, 0 High | Yes |
| 9 | Documentation corrections applied (MED-006) | **PASS** | HANDOVER now documents @noble E2EE accurately | No |
| 10 | Release gate checklist exists (MED-005) | **PASS** | This section defines it; [`HANDOVER.md` § Release Gates](HANDOVER.md) references it as the formal pass/fail criteria | Yes |

---

## Re-verification 2026-05-27

The full checklist was re-walked against `main @ dbe09fb` using non-mutating commands only. All 10 gates PASS.

| Gate | Evidence (2026-05-27) |
|------|------------------------|
| 1 | CI workflow runs `26480257961` + E2E Security `26480257963` + Push on main `26480257348` — all `success` for `dbe09fb` |
| 2 | [`backend/.cargo/audit.toml`](../backend/.cargo/audit.toml) — one ignore (RUSTSEC-2023-0071), reviewed 2026-05-26 |
| 3 | [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) lines 105-107 — `cargo clippy -- -D warnings` |
| 4 | [`.github/workflows/security-scans.yml`](../.github/workflows/security-scans.yml) lines 46-52 — `npm audit --omit=dev --audit-level=high` for frontend and marketing |
| 5 | [`.github/workflows/security-scans.yml`](../.github/workflows/security-scans.yml) lines 54-58 — `gitleaks/gitleaks-action@v2` full-history scan |
| 6 | [`.github/workflows/security-scans.yml`](../.github/workflows/security-scans.yml) lines 72-81 — `trivy-action@v0.36.0` (minor drift from `v0.35.0` recorded in original audit; no functional impact) |
| 7 | [`.github/workflows/e2e-pr-smoke.yml`](../.github/workflows/e2e-pr-smoke.yml) — `--grep "@smoke"` smoke suite present and passing on frontend/backend-touching PRs |
| 8 | [`dev docs/SECURITY_AUDIT.md`](SECURITY_AUDIT.md) revision 2 — 0 Critical, 0 High; STABILIZATION_AUDIT has no open Critical or High findings either |
| 9 | HANDOVER Section 2 documents `@noble/curves + @noble/hashes` accurately (MED-006 resolved) |
| 10 | [`HANDOVER.md` § Release Gates](HANDOVER.md) (line 584 area) references this checklist as the authoritative pass/fail criteria (MED-005 resolved) |

GitHub Security tab on `main` is also clean as of 2026-05-27: 0 open across Dependabot, code scanning, and secret scanning (see [`SECURITY_AUDIT.md` § 2.4](SECURITY_AUDIT.md)).

---

## Findings

### MED-005 — Release Gates Undefined (RESOLVED 2026-05-27)

**Pillar:** Infrastructure / Documentation
**Severity:** Medium
**Component:** `dev docs/HANDOVER.md:4`
**Regression from freeze:** No (never existed)
**Blocking production automation:** Yes
**Status:** RESOLVED — HANDOVER.md § Release Gates (line 584 area) now references this document's Release Gate Checklist as the formal pass/fail criteria. Re-verified 10/10 PASS on 2026-05-27.

#### Evidence

HANDOVER.md line 4:
```
CI/CD production deploy/release automation is paused until the documented release gates are proven
```

HANDOVER.md line 669:
```
Backend production deploy automation paused during stabilization; manual Fly.io promotion only after the release gate is reinstated
```

No document in the repository defines what "the documented release gates" or "the release gate" actually are. The closest approximation is the 8-item priority list at HANDOVER.md lines 654-663, which mixes launch tasks (macOS DMG build, iOS build, Google Play submission) with operational prerequisites. No pass/fail criteria, no automated verification, no ownership assignment.

#### Impact

Without defined gates, production promotion depends entirely on implicit developer judgment. The stabilization phase cannot end because there is no measurable definition of "done."

#### Fix

Adopt the Release Gate Checklist in Section 2 of this document as the formal gate definition. Add a reference in HANDOVER.md:

```markdown
### Release Gates
See `dev docs/STABILIZATION_AUDIT.md § Release Gate Checklist` for the formal pass/fail criteria
that must be satisfied before `flyctl deploy` automation resumes.
```

#### Verification

```bash
grep -F "Release Gate" "dev docs/HANDOVER.md"
```

---

### MED-006 — Obsolete Crypto-Library References in HANDOVER.md

**Pillar:** Documentation
**Severity:** Medium
**Component:** `dev docs/HANDOVER.md` lines 31, 60, 279, 379, 590
**Regression from freeze:** No (pre-existing, never corrected in HANDOVER)
**Blocking production automation:** Yes (blocks developer onboarding and public launch)

#### Evidence

Five locations in an earlier HANDOVER revision referenced an obsolete crypto-library architecture:

| Line | Current text |
|------|-------------|
| 31 | obsolete WASM-based E2EE architecture diagram text |
| 60 | obsolete native-protocol library note |
| 279 | obsolete C FFI guidance |
| 379 | obsolete signal wrapper directory description |
| 590 | obsolete WASM initialization latency estimate |

The actual implementation uses `@noble/curves` (Ed25519, X25519) and `@noble/hashes` (HKDF, HMAC-SHA256) - pure TypeScript with zero WASM. This is already correctly documented in:
- `wiki/E2EE-Implementation.md:1-7` ("implemented entirely in the frontend using `@noble/curves` and `@noble/hashes`")
- `wiki/Architecture.md:59` ("`@noble/curves + @noble/hashes | Pure JS, audited, no WASM`")

#### Impact

A developer reading the HANDOVER (the primary onboarding document) will:
- Incorrectly assume WASM is required, complicating WebView build configurations
- Attempt WASM caching optimizations that are unnecessary
- Budget for a nonexistent ~200ms WASM init latency
- Misunderstand the security audit scope for the actual pure TypeScript crypto implementation

#### Fix

Update HANDOVER.md at all 5 locations. Use the canonical phrasing from `wiki/E2EE-Implementation.md`:

| Line | Correct text |
|------|-------------|
| 31 | `@noble (E2EE) \| @noble (E2EE) \| @noble (E2EE)` |
| 60 | `@noble/curves + @noble/hashes` is pure TypeScript; runs on every platform including WebViews |
| 279 | Remove obsolete crypto-library reference; no C FFI is used in this codebase |
| 379 | `signal/ <- @noble/curves E2EE implementation + keystore` |
| 590 | `@noble/curves init (<5ms)` \| First message not delayed \| Pure JS, no WASM module loading |

#### Verification

```bash
grep -i "obsolete crypto-library" "dev docs/HANDOVER.md"
# Expected: 0 matches
```

---

### LOW-010 — Floating Docker Base Image Tag

**Pillar:** Infrastructure
**Severity:** Low
**Component:** `backend/Dockerfile:2,9`
**Regression from freeze:** Unknown (may have drifted during freeze)
**Blocking production automation:** No

#### Evidence

```dockerfile
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS planner   # line 2
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS builder    # line 9
```

The `latest` tag is a floating reference. A Docker build today may use a different Rust toolchain version than a build next week, producing non-reproducible binaries.

#### Impact

Silent toolchain drift. A new `cargo-chef` release could introduce breaking changes or a different Rust version without any visible change to the repository.

#### Fix

Pin to a specific digest or version tag:

```dockerfile
FROM lukemathwalker/cargo-chef:0.1.68-rust-1.80-alpine3.21 AS planner
```

Or use a digest:

```dockerfile
FROM lukemathwalker/cargo-chef@sha256:<current-digest> AS planner
```

Memory impact: none. Build time impact: none.

#### Verification

```bash
docker build --no-cache -t yapper-test backend/
# Verify: same binary hash on consecutive builds
```

---

### LOW-011 — Hub Cache TTLs Without Jitter

**Pillar:** Performance
**Severity:** Low
**Component:** `backend/src/hub.rs:118,120`
**Regression from freeze:** No (original design)
**Blocking production automation:** No

#### Evidence

```rust
const TRUST_CACHE_TTL: Duration = Duration::from_secs(60);      // line 118
const MEMBERSHIP_CACHE_TTL: Duration = Duration::from_secs(300); // line 120
```

All cache entries for a given TTL class expire on the same boundary. The GC task at `main.rs:147-162` runs every 5 minutes and calls `hub.gc_caches()` which retains entries younger than the TTL (`hub.rs:197-203`).

#### Impact

When multiple users join simultaneously (e.g., a server invite link shared publicly), their membership cache entries are created at nearly the same instant and expire together. The next request from each user triggers a simultaneous DB query storm. At 500 members with a 300-second TTL, this could produce 500 concurrent `SELECT 1 FROM server_memberships` queries on a single Neon connection.

In practice, this is mitigated by DashMap's per-key independent expiry and the fact that requests arrive at different times. The risk is theoretical at current scale but becomes meaningful at 500+ concurrent users.

#### Fix

Add jitter to cache insertion:

```rust
use rand::Rng;

fn jittered_ttl(base: Duration) -> Duration {
    let jitter = rand::thread_rng().gen_range(0..=(base.as_secs() / 10));
    base + Duration::from_secs(jitter)
}
```

Memory impact: negligible (~8 bytes per entry for jittered timestamp). Existing test coverage: `hub.rs` unit tests at lines 2220+ cover cache behavior but do not test TTL jitter.

#### Verification

```bash
cargo test -p yapper-server hub::tests -- --nocapture
```

---

### INFO-005 — Migration Sequence Number Duplication

**Pillar:** Infrastructure
**Severity:** Informational
**Component:** `backend/migrations/`
**Regression from freeze:** No (pre-existing)
**Blocking production automation:** No

#### Evidence

Three sequence numbers appear twice with different date prefixes:

| Sequence | File 1 | File 2 |
|----------|--------|--------|
| 000019 | `20260304000019_bots.sql` | `20260306000019_multidevice_e2ee.sql` |
| 000020 | `20260304000020_premium.sql` | `20260307000020_dm_envelope_msg_num.sql` |
| 000031 | `20260324000031_deleted_account_retention.sql` | `20260328000031_fix_message_ciphertext_xor_plaintext.sql` |

#### Impact

None. `sqlx::migrate!()` sorts by the full filename (date prefix + sequence) lexicographically. All 35 migrations execute in the correct order. This is a naming hygiene issue only.

#### Fix

No action required. Future migrations should use unique sequence numbers to avoid confusion.

---

### INFO-006 — Alpine Version Discrepancy in Documentation

**Pillar:** Documentation
**Severity:** Informational
**Component:** `backend/Dockerfile:21`
**Regression from freeze:** Yes (Alpine upgraded during freeze)
**Blocking production automation:** No

#### Evidence

HANDOVER.md (prompt context) references Alpine 3.19. The Dockerfile runtime stage uses:

```dockerfile
FROM alpine:3.21    # line 21
```

#### Impact

None. Alpine 3.21 is a newer, actively supported version. The upgrade is beneficial (security patches).

#### Fix

Update any documentation that references a specific Alpine version to say 3.21. The Dockerfile is the source of truth.

---

### INFO-007 — Secrets Documentation Gap in fly.toml

**Pillar:** Infrastructure
**Severity:** Informational
**Component:** `backend/fly.toml:27-43`
**Regression from freeze:** No (pre-existing)
**Blocking production automation:** No

#### Evidence

`fly.toml` lists 12 secrets in comments (lines 27-43). The backend code references additional environment variables that are not listed:

| Variable | Used In | Required? |
|----------|---------|-----------|
| `APPLE_CLIENT_ID` | `auth/oauth.rs:743` | Optional (Apple OAuth) |
| `APPLE_CLIENT_SECRET` | `auth/oauth.rs:765` | Optional (Apple OAuth) |
| `SENTRY_DSN` | `main.rs:100` | Optional (defaults to empty, disables Sentry) |
| `HUBSPOT_ACCESS_TOKEN` | `support/mod.rs:360` | Optional (support tickets) |
| `HUBSPOT_CLIENT_SECRET` | `support/mod.rs:469` | Optional (webhook verification) |
| `FIREBASE_SERVICE_ACCOUNT_PATH` | notifications module | Optional (push notifications) |
| `EMAIL_FROM` | `auth/handlers.rs:377` | Optional (defaults to `Yapper <hello@yapperhq.com>`) |

All missing variables gracefully default or are only needed for optional features. No runtime crash occurs from their absence.

#### Impact

A new operator deploying Yapper would not know these secrets exist without reading the source code. Feature-complete deployment requires consulting both `fly.toml` and `.env.example`.

#### Fix

Add the missing variables to the `fly.toml` comments section:

```toml
# APPLE_CLIENT_ID          (optional: Apple Sign-In)
# APPLE_CLIENT_SECRET       (optional: Apple Sign-In)
# SENTRY_DSN                (optional: error monitoring)
# HUBSPOT_ACCESS_TOKEN      (optional: support tickets)
# HUBSPOT_CLIENT_SECRET     (optional: webhook verification)
# FIREBASE_SERVICE_ACCOUNT_PATH  (optional: push notifications)
# EMAIL_FROM                (optional: defaults to hello@yapperhq.com)
```

---

### INFO-008 — WebSocket Token Documentation Contradiction Resolved

**Pillar:** Security
**Severity:** Informational
**Component:** `backend/src/hub.rs:573,914`
**Regression from freeze:** No
**Blocking production automation:** No

#### Evidence

The API Reference documents `wss://api.yapperhq.com/ws?token=<access_token>` (token in query string). The Security documentation states tokens must never appear in query strings.

The actual implementation resolves this in favor of security. At `hub.rs:573`:

```rust
/// Authentication happens post-upgrade: the first message must be `{ "type": "auth", "token": "..." }`.
```

At `hub.rs:914`:

```rust
WsInbound::Auth { token } => validate_ws_token(&token, state).await,
```

The token is transmitted inside the first WebSocket frame after the HTTP upgrade completes, never in the query string. This means the token does not appear in Cloudflare access logs, Fly.io application logs, or browser history.

#### Impact

None (the implementation is correct). The API Reference documentation should be updated to reflect the actual first-frame authentication pattern.

#### Fix

Update `docs/api.md` WebSocket section to document the first-frame auth pattern:

```markdown
Connect to `wss://api.yapperhq.com/ws` (no query parameters).
Send as first message: `{"type": "auth", "token": "<access_token>"}`.
```

---

## Findings Summary

| ID | Pillar | Severity | Blocking | Effort |
|----|--------|----------|----------|--------|
| MED-005 | Infrastructure/Docs | Medium | Yes | 1 hour (define + document gates) |
| MED-006 | Documentation | Medium | Yes | 30 min (5 text replacements) |
| LOW-010 | Infrastructure | Low | No | 15 min (pin Docker tag) |
| LOW-011 | Performance | Low | No | 1 hour (add jitter + test) |
| INFO-005 | Infrastructure | Info | No | None (cosmetic, no action) |
| INFO-006 | Documentation | Info | No | 5 min (update version reference) |
| INFO-007 | Infrastructure | Info | No | 15 min (add comments) |
| INFO-008 | Security | Info | No | 15 min (update API docs) |

**Totals:** 0 Critical, 0 High, 2 Medium, 2 Low, 4 Informational

---

## Pillar-by-Pillar Verification Summary

### Pillar 1: Infrastructure and CI/CD Release Gate Verification

| Component | Status | Evidence |
|-----------|--------|----------|
| `auto_stop_machines = false` | PASS | `fly.toml:7` |
| `min_machines_running = 1` | PASS | `fly.toml:9` |
| Non-root Docker user | PASS | `Dockerfile:24-25` (`adduser -S yapper`), `Dockerfile:37` (`USER yapper`) |
| Alpine runtime image | PASS | `Dockerfile:21` (`alpine:3.21`) |
| SQLX_OFFLINE=true in build | PASS | `Dockerfile:17` |
| Argon2id concurrency semaphore | PASS | `auth/service.rs:18-19` — `Semaphore::new(2)`, peak 128MB on 256MB VM |
| DB keepalive (Neon anti-suspend) | PASS | `db.rs:24-36` — 240s interval `SELECT 1` |
| Health endpoint verifies DB | PASS | `lib.rs:221-238` — 2s timeout, 503 on failure |
| CI jobs parallel | PASS | `ci.yml` has no `needs:` chain between `backend`, `frontend`, `marketing` |
| PR smoke uses `@smoke` tag | PASS | `e2e-pr-smoke.yml:162` — `--grep "@smoke" --workers=4` |
| Trivy action version | PASS | `security-scans.yml:95` — `trivy-action@v0.35.0` |
| cargo-audit allowlist | PASS | `.cargo/audit.toml:12-29` — 3 advisories with rationale and 2026-04-06 review dates |
| Migrations applied (35 total) | PASS | `ls backend/migrations/` — 001 through 034 + fix_031 |
| Deploy jobs paused | PASS | `ci.yml:202-255` — deploy jobs commented out |

### Pillar 2: Security Audit

| Component | Status | Evidence |
|-----------|--------|----------|
| Canvas RBAC (23 endpoints) | PASS | `canvas/service.rs:16,60,87` — `require_member`, `require_admin_or_dj`, `require_server_admin` |
| Canvas input validation | PASS | `canvas/types.rs` — `deny_unknown_fields` + `#[validate]` on all DTOs |
| Canvas size caps | PASS | `canvas/types.rs:9-48` — queue=50, polls=5, options=6, reactions=500, pins=3, DJs=20 |
| WS token not in query string | PASS | `hub.rs:573,914` — first JSON frame auth |
| CSRF coverage (9 exemptions) | PASS | `csrf.rs:66-79` — auth endpoints + webhooks only |
| Argon2id + spawn_blocking | PASS | `auth/service.rs:102-114,146-160` |
| Image processing + spawn_blocking | PASS | `users/mod.rs:2340,2363`, `emojis/mod.rs:242`, `discord/mod.rs:288` |
| PBKDF2 iterations | PASS | `backup.ts:27` — 1,200,000 iterations (12x OWASP minimum) |
| GDPR deletion cascade | PASS | `users/mod.rs:555-775` — 25+ tables in atomic transaction, 30-day hold, retention worker |
| Sentry PII scrubbing | PASS | `main.rs:34-91` — headers, emails, tokens, passwords, breadcrumbs |
| No secrets in .env.example | PASS | `.env.example:1-52` — all placeholder values |
| No personal paths in codebase | PASS | `grep -r "C:\\Users\\rajma"` — 0 results (LOW-008 resolved in rev 6) |

### Pillar 3: E2EE and Cryptographic Implementation

| Component | Status | Evidence |
|-----------|--------|----------|
| X3DH key agreement | PASS | `x3dh.ts` — correct DH concatenation, trivial key rejection, SPK signature verification |
| OPK-absent fallback | PASS | `x3dh.ts` — 3-DH when OPK unavailable (per Signal spec) |
| Double Ratchet bounds | PASS | `ratchet.ts:17-19` — MAX_FORWARD_SKIP=128, MAX_SKIPPED_KEYS=512, MAX_SEEN_MESSAGES=1024 |
| Sender Key ECIES distribution | PASS | `sender_keys.ts:100-170` — Ed25519 identity signature, AES-256-GCM |
| Channel preparation local-first | PASS | `index.ts:943-968` — `_preparedChannels` + `ks.loadSenderKey()` before network |
| Server membership validation | PASS | `keys/service.rs:306-323` — sender + recipient membership + device trust checked |
| Safety number determinism | PASS | `index.ts:977-992` — per-user SHA-256(dhPub \|\| sigPub), 6 groups of 5 digits |
| Media encryption IV | PASS | `mediaEncrypt.ts:57-58` — `crypto.getRandomValues(new Uint8Array(12))` |
| R2 object key unpredictable | PASS | `media/r2.rs:107` — `media/{type}/{uuid}.bin` (UUID-based) |
| Key bundle device binding | PASS | `keys/handlers.rs:46-47` — server resolves device ID, caller-supplied ignored |
| Crypto library correct | PASS | `@noble/curves` + `@noble/hashes` (pure TS, no WASM) |

### Pillar 4: Feature Integrity and Stale Logic Detection

| Component | Status | Evidence |
|-----------|--------|----------|
| COPPA consent tracking | PASS | `users/mod.rs:738` — `coppa_consent_verified_at` set on creation |
| Parental approval gates | PASS | `parental/mod.rs` — friend request + server join approval workflows |
| Canvas WS events (14/14) | PASS | `canvas/service.rs` broadcasts all 14 types; `canvas.ts:446-653` handles all in switch |
| Canvas state hydration | PASS | `canvas/handlers.rs` — GET returns music + polls + clips + events snapshot |
| Emoji pixel bomb protection | PASS | `emojis/mod.rs:244-260` — `into_dimensions()` header-only read |
| Emoji shortcode XSS prevention | PASS | `emojis/mod.rs:49` — regex `^[a-z0-9_]{2,32}$` |
| Emoji size limit | PASS | `emojis/mod.rs:58` — `DefaultBodyLimit::max(MAX_EMOJI_BYTES)` (256KB) |
| Screen Time endpoints | PASS | Frontend calls match backend handlers (GET + PATCH screentime) |
| Discord Import | PASS | `discord/mod.rs` + `bots/mod.rs` — import routes implemented |
| No TODO/FIXME markers | PASS | 0 matches in `backend/src/` and `frontend/src/` |

### Pillar 5: Performance and Operational Readiness

| Component | Status | Evidence |
|-----------|--------|----------|
| Fan-out mechanism | PASS | `hub.rs:423` — sequential `try_send()` (non-blocking, microsecond per call), capped at MAX_FANOUT_MEMBERS=500 |
| Neon keepalive | PASS | `db.rs:24-36` — 240s interval prevents 5-min auto-suspend |
| Playwright parallel | PASS | `playwright.config.ts:20,23` — `fullyParallel: true`, `workers: 4` (CI) |
| GC task | PASS | `main.rs:147-162` — 5-min interval, rate limiter + cache cleanup |
| Retention worker | PASS | `main.rs:185-195` — 6-hour interval, GDPR data purge |
| WS rate limiting | PASS | `hub.rs` — 5/sec per user burst 20, 10/sec per IP on upgrade |
| WS frame size limit | PASS | `hub.rs:597-598` — MAX_WS_FRAME_SIZE=64KB at protocol layer |
| Connection limit | PASS | `hub.rs:234` — MAX_CONNECTIONS_PER_USER=5 |
| Concurrency limits | PASS | `fly.toml:12-14` — hard=500, soft=400 connections |

### Pillar 6: Documentation Drift Assessment

| Component | Status | Evidence |
|-----------|--------|----------|
| Obsolete crypto-library references | **PASS** | HANDOVER.md corrected to @noble E2EE — see MED-006 |
| Release gate definition | **FAIL** | Not defined anywhere — see MED-005 |
| Personal path leak | PASS | 0 results for personal Windows absolute paths in codebase (LOW-008 resolved) |
| API reference completeness | PASS | `wiki/Architecture.md:59` lists all modules; `docs/api.md` covers v2 auth + devices |
| Cargo audit documentation | PASS | Single source of truth at `backend/.cargo/audit.toml` (LOW-009 resolved) |
| .env.example completeness | PASS | All required vars documented; optional vars have defaults |

---

## Integration Test Status

All three previously-known integration test failures have been resolved in commit `4c6bc9a` ("test: stabilize suite and remediate test infrastructure").

### Test 1: `auth_register_login_me_logout`

- **File:** `backend/tests/integration/auth.rs:125-169`
- **Previous failure:** 403 on logout (CSRF hypothesis)
- **Current status:** PASSING. The `TestClient.delete()` method routes through `mutating_request()` (`mod.rs:186-190`) which attaches both the `X-CSRF-Token` header and `csrf_token` cookie. The test infrastructure correctly propagates CSRF credentials on all mutating requests.

### Test 2: `keys_upload_rejects_batches_over_the_maximum_size`

- **File:** `backend/tests/integration/keys.rs:197-234`
- **Previous failure:** Stale assertion against exact error text
- **Current status:** PASSING. Lines 224-232 now use substring matching (`error_text.contains("one-time prekeys")`) with a comment noting the en dash encoding variance.

### Test 3: `presence_hides_last_seen_for_peers_and_keeps_self_view`

- **File:** `backend/tests/integration/privacy.rs:14-75`
- **Previous failure:** 500 error (NULL into NOT NULL on first privacy write)
- **Current status:** PASSING. The regression note at line 35-36 confirms the upsert handler was fixed. The test asserts 204 on the privacy update (line 43-48).

**No integration test repair plan is needed.**

---

## Documentation Correction Register

| # | Document | Line(s) | Current Text | Correct Text | Priority |
|---|----------|---------|-------------|-------------|----------|
| 1 | HANDOVER.md | 31 | obsolete WASM-based architecture diagram text | `@noble (E2EE) \| @noble (E2EE) \| @noble (E2EE)` | Resolved |
| 2 | HANDOVER.md | 60 | obsolete native-protocol library note | `@noble/curves + @noble/hashes` is pure TypeScript; runs on every platform | Resolved |
| 3 | HANDOVER.md | 279 | obsolete C FFI guidance | Remove obsolete reference; no C FFI in use | Resolved |
| 4 | HANDOVER.md | 379 | obsolete signal wrapper directory description | `signal/ <- @noble/curves E2EE implementation + keystore` | Resolved |
| 5 | HANDOVER.md | 590 | obsolete WASM initialization latency estimate | `@noble/curves init (<5ms) \| Not delayed \| Pure JS, no WASM loading` | Resolved |
| 6 | fly.toml | 27-43 | 12 secrets listed | Add 7 missing optional secrets (APPLE, SENTRY, HUBSPOT, FIREBASE, EMAIL_FROM) | Recommended |

---

## Previously Remediated Findings (Verified, Not Re-Flagged)

These findings from prior audits were verified as resolved during this assessment. They are documented here for completeness and are not re-raised.

| Original ID | Description | Resolved In | Verification Location |
|-------------|-------------|-------------|----------------------|
| HIGH-001 | Argon2id OOM risk (no semaphore) | HANDOVER rev 5 | `auth/service.rs:18-19` — `Semaphore::new(2)` |
| HIGH-002 | 15+ silent `catch(() => {})` blocks | HANDOVER rev 5 | Signal + presence catches now log via `console.warn` |
| HIGH-003 | LiveCanvas isAdmin not wired | HANDOVER rev 5 | Wired to `serversStore` |
| MED-001 | `/health` returned 200 unconditionally | HANDOVER rev 5 | `lib.rs:221-238` — DB check with 2s timeout, returns 503 |
| MED-003 | hub.rs silent error drops | HANDOVER rev 5 | `tracing::warn!` + `tracing::error!` on all error paths |
| MED-004 | hub.rs module split needed | HANDOVER rev 5 | Documented as post-launch; SAFETY comments added |
| LOW-002 | NonZeroU32 missing SAFETY comment | HANDOVER rev 5 | SAFETY comment present |
| LOW-006 | Wiki E2EE doc / code mismatch | HANDOVER rev 6 | 1.2M PBKDF2 iterations, SHA-256 safety numbers documented |
| LOW-007 | No emergency ops runbook | HANDOVER rev 6 | `docs/deployment.md` — Fly.io ops runbook added |
| LOW-008 | Personal Windows path leak | HANDOVER rev 6 | 0 results for personal Windows absolute paths in codebase |
| LOW-009 | Cargo audit ignores split across files | HANDOVER rev 6 | `backend/.cargo/audit.toml` is single source of truth |

---

## Production Automation Conditions

The following must be completed in order before `flyctl deploy` automation resumes:

1. **Keep documentation corrections 1-5 closed** from the Documentation Correction Register (MED-006). HANDOVER now uses @noble/curves + @noble/hashes terminology.

2. **Adopt the Release Gate Checklist** from Section 2 of this document (MED-005). Add a reference in HANDOVER.md pointing to this checklist as the formal gate definition.

3. **Verify all CI checks pass on `main`** after the documentation changes are merged. All 5 parallel jobs (docs-sync, backend, frontend, marketing, security-scans) must be green.

4. **Run E2E smoke suite** (`e2e-pr-smoke.yml`) against staging to confirm no regressions from the documentation-only changes.

5. **Uncomment the deploy jobs** in `.github/workflows/ci.yml:202-255` and configure with appropriate environment protection rules (require manual approval for production, auto-deploy to staging).

6. **Perform one manual `flyctl deploy`** to production to verify the current `main` branch deploys cleanly before enabling automation.

Once all 6 conditions are met, CI/CD production deploy automation can safely resume.

---

## Appendix: Cargo Audit Ignore List (Acknowledged)

The following advisory is tracked in `backend/.cargo/audit.toml:12-18` with documented rationale and quarterly review date. It is not re-flagged in this audit per the absolute constraints.

| Advisory | Crate Path | Rationale | Next Review |
|----------|-----------|-----------|-------------|
| RUSTSEC-2023-0071 | `rsa` via `sqlx-mysql` and `jsonwebtoken` | sqlx-mysql path unreachable; RS256 JWT signing/verification path accepted pending upstream or auth-library replacement | 2026-08-26 |
