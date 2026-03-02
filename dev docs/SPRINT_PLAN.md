# YAPPER — Sprint Plan

> Generated: 2026-02-27
> Source: `quizzical-yawning-starfish.md` (Full Implementation Plan)
> Methodology: 2-week sprints, each with clear deliverables, acceptance criteria, and team assignments

---

## Sprint Calendar Overview

| Sprint | Weeks | Phase(s) | Theme | Status |
|--------|-------|----------|-------|--------|
| **S0** | W1–W2 | Pre-Phase + Phase 0 | Setup + Marketing Site | In Progress |
| **S1** | W3–W4 | Phase 1 | Repo Scaffolding & Dev Environment | ✅ Complete |
| **S2** | W5–W6 | Phase 2 | Authentication & User System | ✅ Complete |
| **S3** | W7–W8 | Phase 3 | Signal Protocol & E2EE (1:1 DMs) | ✅ Complete |
| **S4** | W9–W10 | Phase 4 | Servers, Channels & Group E2EE | Not Started |
| **S5** | W11–W12 | Phase 5 + Phase 6 | Media Messages + Real-Time Features | Not Started |
| **S6** | W13–W14 | Phase 7 + Phase 8 | Live Canvas + Explore Page | Not Started |
| **S7** | W15–W16 | Phase 9 + Phase 10 | Profiles + Parental Controls | Not Started |
| **S8** | W17–W18 | Phase 11 + Phase 12 | Screen Time + Discord Integration | Not Started |
| **S9** | W19–W20 | Phase 13 + Phase 14 | Emojis + Settings | Not Started |
| **S10** | W21–W22 | Phase 15 + Phase 16 | Desktop Polish + Security Audit | Not Started |
| **S11** | W23–W24 | Phase 17 + Phase 18 | Premium Prep + Launch | Not Started |

**Total: ~24 weeks (6 months) to MVP launch.**

---

## Team Roles (Reference)

| Role | Abbreviation | Responsibility |
|------|-------------|----------------|
| Backend Engineer | **BE** | Rust/Axum API, DB migrations, WebSocket hub |
| Frontend Engineer | **FE** | SvelteKit UI, Signal WASM, stores |
| Full-Stack / DevOps | **FS** | Infrastructure, CI/CD, Tauri/Capacitor, deployment |
| Designer | **DS** | UI mockups, brand assets, OG images |

> For a solo developer or small team: one person fills multiple roles. The labels indicate *type of work*, not headcount.

---

## S0 — Setup + Marketing Site (Weeks 1–2)

**Goal:** All accounts created, tools installed, marketing site live on Cloudflare Pages with working wishlist signup.

### Week 1: Machine & Account Setup (Pre-Phase)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Verify Rust toolchain (`rustc 1.93+`), install `cargo-watch`, `sqlx-cli` | FS | [x] |
| 2 | Install Tauri CLI v2, Capacitor CLI | FS | [x] |
| 3 | Install Android Studio (SDK API 26+, NDK) | FS | [x] |
| 4 | **Apply for Apple FamilyControls entitlement** (1–4 week lead time) | FS | [ ] |
| 5 | Create Cloudflare account: R2 bucket, D1 database, KV namespace, Pages project, Workers | FS | [x] |
| 6 | Create Fly.io account, install `flyctl`, `fly auth login` | FS | [x] |
| 7 | Create Neon account, new project, copy connection string | FS | [x] |
| 8 | Create Firebase project → FCM → download `service-account.json` | FS | [x] |
| 9 | Create Discord Developer Application → OAuth2 credentials | FS | [x] |
| 10 | Register Apple Developer + Google Cloud OAuth credentials | FS | [~] |
| 11 | Create Resend account → copy API key | FS | [x] |
| 12 | Purchase domain (`yapperhq.com` or alternative) on Porkbun | FS | [x] |

### Week 2: Marketing Site (Phase 0)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | `npm create astro@latest marketing` with `@astrojs/svelte` integration | FE | [x] |
| 2 | Create `Base.astro` layout: `<head>` with OG tags, self-hosted fonts, global CSS vars | FE | [x] |
| 3 | Create `global.css`: brand colors, typography scale, dark theme variables | DS/FE | [x] |
| 4 | Build `Hero.astro`: headline, subheadline, CSS 3D sphere animation, email CTA | FE | [x] |
| 5 | Build `FeatureGrid.astro`: 8 feature cards with icons and hook lines | FE | [x] |
| 6 | Build `HowItWorks.astro`: 3-step horizontal stepper | FE | [x] |
| 7 | Build `SafetySection.astro`: E2EE + parental controls two-column | FE | [x] |
| 8 | Build `PlatformBadges.astro`, `PricingPreview.astro`, `Footer.astro` | FE | [x] |
| 9 | Build `WishlistForm.svelte`: email input, submit, success/error states (client island) | FE | [x] |
| 10 | Build `FAQAccordion.svelte`: 6 questions with expand/collapse | FE | [x] |
| 11 | Assemble `index.astro`: import all sections | FE | [x] |
| 12 | Write `worker/wishlist.js`: email validation → D1 insert → KV counter → Resend confirm | BE | [x] |
| 13 | Write `wrangler.toml`: D1 + KV bindings | BE | [x] |
| 14 | Run D1 migration: create `wishlist` + `counter` tables | BE | [x] |
| 15 | Set Resend API key secret: `wrangler secret put RESEND_API_KEY` | BE | [x] |
| 16 | Create `og-image.png` (1200x630) and `favicon.svg` | DS | [x] |
| 17 | Deploy: Cloudflare Pages (Astro) + `wrangler deploy` (Worker) | FS | [x] |
| 18 | Configure custom domain in Cloudflare DNS | FS | [x] |
| 19 | Run Lighthouse audit — target all scores >= 95 | FE | [x] |

### S0 Acceptance Criteria
- [ ] All service accounts created and credentials stored securely
- [x] Marketing site live at production URL (https://yapperhq.com)
- [x] Wishlist form submits → email appears in D1 → confirmation email received
- [ ] Live signup counter renders server-side
- [ ] Lighthouse: Performance >= 95, Accessibility >= 95, Best Practices >= 95, SEO >= 95
- [ ] Apple FamilyControls entitlement application submitted

---

## S1 — Repo Scaffolding & Dev Environment (Weeks 3–4)

**Goal:** Monorepo with runnable backend (health endpoint), frontend dev server, Tauri window, and CI pipeline.

### Week 3: Backend Skeleton

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | `git init` in `yapper/`, create `.gitignore` (Rust + Node + env files) | FS | [x] |
| 2 | Create `docker-compose.yml`: PostgreSQL 16 only (no Redis) | FS | [x] |
| 3 | `cargo init --name yapper-server` in `backend/` | BE | [x] |
| 4 | Add all `Cargo.toml` dependencies (axum, tokio, sqlx, argon2, jsonwebtoken, tower-http, governor, dashmap, tracing, etc.) | BE | [x] |
| 5 | Create `src/main.rs`: Tokio entrypoint, Axum router, `GET /health → 200 OK` | BE | [x] |
| 6 | Create `src/error.rs`: `AppError` type with `impl IntoResponse` | BE | [x] |
| 7 | Create `src/db.rs`: `sqlx::PgPool` setup + `PgListener` LISTEN/NOTIFY helper | BE | [x] |
| 8 | Create `src/hub.rs`: `Arc<DashMap<UserId, HashMap<DeviceId, mpsc::Sender>>>` skeleton + WS upgrade handler | BE | [x] |
| 9 | Add `tower-http` middleware stack: CORS, security headers (HSTS, CSP, X-Frame-Options, nosniff), gzip, tracing | BE | [x] |
| 10 | Add `governor` rate limiting middleware (per-IP) | BE | [x] |
| 11 | Create `migrations/` directory with initial `20260301120000_users.sql` and `20260301120001_sessions.sql` | BE | [x] |
| 12 | Create `fly.toml`: `shared-cpu-1x`, `256mb` | BE | [x] |
| 13 | Create `Dockerfile`: multi-stage (`rust:alpine` builder → `scratch` final image) | BE | [x] |
| 14 | Create `.env.example` with all required env vars | FS | [x] |
| 15 | Verify: `docker compose up -d` → PostgreSQL healthy → `cargo run` → `curl localhost:8080/health` → 200 | BE | [x] |

### Week 4: Frontend Skeleton + CI

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | `npx sv create frontend` (SvelteKit, static adapter, TypeScript) | FE | [x] |
| 2 | Configure `svelte.config.js` with `adapter-static` | FE | [x] |
| 3 | Configure `vite.config.ts` with API proxy to backend | FE | [x] |
| 4 | `cd frontend && npm run tauri init` — Tauri v2 shell | FE | [x] |
| 5 | Configure `tauri.conf.json`: bundle identifier `com.yapperhq.com`, window settings | FE | [x] |
| 6 | `npx cap init Yapper com.yapperhq.com` — Capacitor setup | FE | [x] |
| 7 | `npx cap add ios && npx cap add android` — add platforms | FE | [x] |
| 8 | Create `capacitor.config.ts` with server URL config | FE | [x] |
| 9 | Create `Makefile`: `dev-backend`, `dev-frontend`, `migrate`, `deploy`, `test` targets | FS | [x] |
| 10 | Set up GitHub Actions CI: `cargo test`, `cargo clippy`, `cargo fmt --check`, `cargo audit`, `npm test`, `npm run build` | FS | [x] |
| 11 | Add Vitest + `@testing-library/svelte` to frontend | FE | [x] |
| 12 | Add Playwright for E2E test scaffolding | FE | [ ] |
| 13 | Write first backend test: health endpoint returns 200 | BE | [x] |
| 14 | Write first frontend test: app shell renders | FE | [x] |
| 15 | `fly deploy` → backend live on Fly.io → health check passes | FS | [x] |

### S1 Acceptance Criteria
- [x] `docker compose up -d && cargo run` → health endpoint responds
- [x] `cargo sqlx prepare` → SQL queries verified against live DB
- [x] `npm run dev` → SvelteKit at `localhost:5173`
- [x] `npm run tauri dev` → Tauri window opens with SvelteKit content
- [x] GitHub Actions CI passes on push to `main`
- [x] `fly deploy` → backend accessible on Fly.io
- [x] At least 1 backend test + 1 frontend test passing

---

## S2 — Authentication & User System (Weeks 5–6)

**Goal:** Full register → verify email → login → JWT refresh → social OAuth → rate limiting.

### Week 5: Backend Auth

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Create `src/auth/mod.rs`, `handlers.rs`, `service.rs`, `middleware.rs` | BE | [x] |
| 2 | Implement `POST /auth/register`: Argon2id hash, email verification token, store user | BE | [x] |
| 3 | Implement `POST /auth/login`: verify password, return RS256 JWT (15min) + HttpOnly refresh cookie | BE | [x] |
| 4 | Implement `POST /auth/refresh`: rotate refresh token with family-based reuse detection | BE | [x] |
| 5 | Implement `DELETE /auth/logout`: invalidate token family, remove session | BE | [x] |
| 6 | Create JWT middleware extractor with `kid` header support for key rotation | BE | [x] |
| 7 | Add CSRF token validation on state-changing endpoints (`X-CSRF-Token` double-submit) | BE | [x] |
| 8 | Add login rate limiting: 5 failed attempts → 15-min lockout (in-memory `DashMap`) | BE | [x] |
| 9 | Implement email verification via Resend: `POST /email/verify` | BE | [x] |
| 10 | Implement password reset via Resend: `POST /email/password-reset` | BE | [x] |
| 11 | Write migrations: `users`, `sessions` tables with all columns from schema | BE | [x] |
| 12 | Write integration tests: register, login, refresh, reuse detection, rate limiting | BE | [x] |

### Week 6: Frontend Auth + OAuth

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Create `src/lib/api/client.ts`: typed fetch wrapper with JWT auto-attach and refresh interceptor | FE | [x] |
| 2 | Create `src/lib/stores/auth.ts`: auth state, token storage, login/logout actions | FE | [x] |
| 3 | Build Sign In page (Screen 7): dark split layout, email+password, social buttons, "Enter the Void" CTA | FE | [x] |
| 4 | Build Sign Up page (Screen 8): handle + email + password (strength meter), social buttons, "Join the Hype" CTA | FE | [x] |
| 5 | Build Onboarding Screen 1 (Screen 9): 3D sphere CSS, "A New Way to Yap", dot pagination | FE | [x] |
| 6 | Build Onboarding Screen 2 (Screen 10): community discovery cards, "Ready to Yap" CTA | FE | [x] |
| 7 | Implement Discord OAuth2 flow: `GET /auth/oauth/discord` → redirect → callback → user created/linked | BE | [x] |
| 8 | Implement Google OAuth2 flow | BE | [x] |
| 9 | Implement Apple Sign-In OAuth2 flow | BE | [ ] |
| 10 | Wire frontend social login buttons to OAuth redirect endpoints | FE | [x] |
| 11 | Create `(auth)` layout group with route guards (redirect if logged in) | FE | [x] |
| 12 | Create `(app)` layout group with route guards (redirect if NOT logged in) | FE | [x] |
| 13 | Write Playwright E2E test: register → verify → login → see dashboard | FE | [ ] |
| 14 | Write component tests: SignIn form validation, SignUp strength meter | FE | [ ] |

### S2 Acceptance Criteria
- [x] Full register → email verification → login flow works end-to-end
- [x] Access token expires → refresh token rotates correctly
- [x] Revoked refresh token replay → entire token family invalidated
- [x] Discord OAuth2 redirect creates/links user account
- [x] Rate limiter blocks after 5 failed login attempts
- [x] All 4 auth screens match mockup designs
- [x] CI passes with new tests

---

## S3 — Signal Protocol & E2EE Core (Weeks 7–8)

**Goal:** Two users can exchange fully E2EE direct messages. Server never sees plaintext.

### Week 7: Key Management + Signal Integration

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migrations: `identity_keys`, `signed_prekeys`, `one_time_prekeys` tables | BE | [x] |
| 2 | Create `src/keys/`: handlers + service for key bundle CRUD | BE | [x] |
| 3 | Implement `POST /api/v1/keys/identity` — upload identity public key | BE | [x] |
| 4 | Implement `POST /api/v1/keys/signed-prekey` — rotate signed prekey | BE | [x] |
| 5 | Implement `POST /api/v1/keys/one-time-prekeys` — batch upload 100 OPKs | BE | [x] |
| 6 | Implement `GET /api/v1/keys/{user_id}` — return key bundle, mark OPK consumed | BE | [x] |
| 7 | Implement `GET /api/v1/keys/one-time-prekey-count` — alert if < 10 | BE | [x] |
| 8 | Install `@noble/curves` + `@noble/hashes` (pure Web Crypto; libsignal-client is NAPI-only) | FE | [x] |
| 9 | Create `src/lib/signal/index.ts`: `setupKeys()`, `encryptDm()`, `decryptDm()`, X3DH + ratchet | FE | [x] |
| 10 | Create `src/lib/signal/keystore.ts`: IndexedDB storage via `idb` | FE | [x] |
| 11 | Implement key generation on first registration + upload to server | FE | [x] |
| 12 | Write backend tests: key upload, bundle fetch, OPK consumption + depletion | BE | [ ] |

### Week 8: WebSocket Hub + DM Messaging

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migrations: `dm_conversations`, `dm_participants`, `messages`, `message_read_receipts` tables | BE | [x] |
| 2 | Implement WebSocket upgrade handler: auth via first message `{ type: "auth", token }`, register in hub | BE | [x] |
| 3 | Implement re-authentication: server sends `re_auth_required` 60s before JWT expiry | BE | [x] |
| 4 | Implement DM message flow: receive encrypted frame → store ciphertext → route to recipient's `mpsc::Sender` → offline fallback to DB | BE | [x] |
| 5 | Implement offline delivery: on connect, query undelivered messages → push all → mark delivered | BE | [x] |
| 6 | Implement WS rate limiting: token bucket per user via `governor` | BE | [x] |
| 7 | Create `src/lib/stores/ws.ts`: WebSocket store with reconnect (exponential backoff) | FE | [x] |
| 8 | Build DM conversation page: `(app)/dm/[conversationId]/+page.svelte` | FE | [x] |
| 9 | Build `MessageList.svelte`: paginated message display with decryption | FE | [x] |
| 10 | Build `MessageInput.svelte`: text input, encrypt via Signal session, send via WS | FE | [x] |
| 11 | Implement X3DH session creation: Alice fetches Bob's bundle → local session → first message | FE | [x] |
| 12 | Implement PIN-based key backup: `PUT /api/v1/keys/backup` (encrypted blob) | BE/FE | [x] |
| 13 | Write E2E test: Alice → Bob DM, verify DB has only ciphertext | BE | [ ] |
| 14 | Write integration tests: WS auth, offline delivery, rate limiting | BE | [ ] |

### S3 Acceptance Criteria
- [x] Alice registers + uploads keys. Bob registers + uploads keys
- [x] Alice fetches Bob's key bundle → X3DH session created locally
- [x] Alice sends encrypted message → DB stores ONLY ciphertext (no plaintext)
- [x] Bob receives via WebSocket → decrypts successfully → plaintext visible in UI
- [x] Offline messages delivered on reconnect
- [x] WS auth rejects invalid/expired tokens
- [x] PIN-based key backup: set PIN → backup → restore on new session
- [ ] CI passes with Signal + WS tests

---

## S4 — Servers, Channels & Group Messaging (Weeks 9–10)

**Goal:** Server CRUD, channel management, E2EE group messages via Sender Keys.

### Week 9: Server & Channel Backend

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migrations: `servers`, `server_memberships`, `channels`, `server_invite_links` tables + indexes | BE | [x] |
| 2 | Create `src/servers/`: handlers + service | BE | [ ] |
| 3 | Implement `POST /api/v1/servers` — create server | BE | [ ] |
| 4 | Implement `GET /api/v1/servers/{id}`, `PATCH /api/v1/servers/{id}` | BE | [ ] |
| 5 | Implement `POST /api/v1/servers/{id}/join` — with child account pending flow intercept | BE | [ ] |
| 6 | Implement `POST /api/v1/servers/{id}/invite` — generate invite code | BE | [ ] |
| 7 | Create `src/channels/`: handlers + service | BE | [ ] |
| 8 | Implement `GET /api/v1/servers/{id}/channels`, `POST /api/v1/servers/{id}/channels` | BE | [ ] |
| 9 | Implement `GET /api/v1/channels/{id}/messages` — paginated ciphertext | BE | [ ] |
| 10 | Implement `POST /api/v1/channels/{id}/messages` — store ciphertext | BE | [ ] |
| 11 | Extend hub to fan out channel messages to all connected members | BE | [ ] |
| 12 | Write tests: server CRUD, membership, invite links, channel messages | BE | [ ] |

### Week 10: Sender Keys + Frontend

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Implement Sender Key generation per (channel, device) pair in Signal wrapper | FE | [ ] |
| 2 | Implement Sender Key distribution: on channel join, distribute to all members via individual Signal sessions | FE | [ ] |
| 3 | Implement group encrypt: sender encrypts once with SenderKey chain | FE | [ ] |
| 4 | Implement group decrypt: all members decrypt with sender's SenderKey copy | FE | [ ] |
| 5 | Handle new member join: receive SenderKeys from all existing members | FE | [ ] |
| 6 | Build Server Chat page: `(app)/servers/[id]/channels/[channelId]/+page.svelte` | FE | [ ] |
| 7 | Build `ServerSidebar.svelte`: server list, server icon, unread indicators | FE | [ ] |
| 8 | Build `ChannelList.svelte`: channel names with type icons, active state | FE | [ ] |
| 9 | Build server creation modal: name, icon upload, public/private toggle | FE | [ ] |
| 10 | Build invite link sharing UI: generate + copy link | FE | [ ] |
| 11 | Write E2E test: create server → invite user → both join → encrypted group message | FE | [ ] |
| 12 | Verify: new member joins later → cannot see prior messages (forward secrecy) | BE/FE | [ ] |

### S4 Acceptance Criteria
- [ ] Server created, invite link generated, second user joins via invite
- [ ] Channel message sent → stored as ciphertext → both members decrypt
- [ ] New member joins → cannot see messages sent before their join
- [ ] Sender Key distribution completes for all existing members
- [ ] Server sidebar and channel list render correctly
- [ ] Child account server join is intercepted (pending state stored)

---

## S5 — Media Messages + Real-Time Features (Weeks 11–12)

**Goal:** Audio Yaps, Video Clips (E2EE via R2), typing indicators, read receipts, presence.

### Week 11: E2EE Media (Phase 5)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Create `src/media/`: handlers + `r2.rs` (R2 client via `aws-sdk-s3` with custom endpoint) | BE | [ ] |
| 2 | Implement `POST /api/v1/media/upload-url`: generate R2 pre-signed PUT URL (15-min expiry) | BE | [ ] |
| 3 | Configure R2 bucket CORS: PUT from app origin, GET for public reads | FS | [ ] |
| 4 | Set R2 lifecycle rule: delete objects after 30 days | FS | [ ] |
| 5 | Create `src/lib/signal/mediaEncrypt.ts`: `encryptMedia(blob)` → AES-256-GCM, `decryptMedia(encryptedBlob, key, iv)` | FE | [ ] |
| 6 | Build `YapRecorder.svelte`: MediaRecorder (audio), waveform visualization, encrypt + upload to R2, embed keys in Signal payload | FE | [ ] |
| 7 | Build `ClipRecorder.svelte`: MediaRecorder (video), preview thumbnail, encrypt + upload to R2 | FE | [ ] |
| 8 | Build `YapMessage.svelte`: waveform playback UI with duration | FE | [ ] |
| 9 | Build `ClipMessage.svelte`: video player with decrypt-on-play | FE | [ ] |
| 10 | Write test: record Yap → R2 object is encrypted → recipient decrypts → plays | BE/FE | [ ] |
| 11 | Verify: `messages` table has no media URL or key in plaintext columns | BE | [ ] |

### Week 12: Real-Time Features (Phase 6)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Implement typing indicators in hub: `DashMap<(ChannelId, UserId), JoinHandle>`, auto-stop after 5s | BE | [ ] |
| 2 | Implement typing fan-out: broadcast `typing_start`/`typing_stop` to channel members | BE | [ ] |
| 3 | Implement read receipts: upsert `message_read_receipts` → fan out `read_receipt` event | BE | [ ] |
| 4 | Implement presence: online = exists in hub `DashMap`, `GET /api/v1/users/{id}/presence` | BE | [ ] |
| 5 | Implement away detection: no WS activity for 5 min → away flag; update `last_seen_at` on disconnect | BE | [ ] |
| 6 | Build `TypingIndicator.svelte`: animated dots, "X and Y are typing..." | FE | [ ] |
| 7 | Build `ReadReceipt.svelte`: "Read" timestamp (DMs), "3 reads" count (channels) | FE | [ ] |
| 8 | Integrate IntersectionObserver for viewport-based read receipt sending | FE | [ ] |
| 9 | Add presence indicators to user avatars (green dot = online, gray = offline) | FE | [ ] |
| 10 | Update `ws.ts` store: handle typing, read receipt, presence events | FE | [ ] |
| 11 | Write tests: typing auto-stop, read receipt dedup, presence on/off | BE | [ ] |

### S5 Acceptance Criteria
- [ ] Audio Yap recorded → encrypted → uploaded to R2 → received → decrypted → plays
- [ ] Video Clip same flow works
- [ ] R2 object cannot be played directly (encrypted blob)
- [ ] Typing indicator appears within 100ms, auto-stops after 5s inactivity
- [ ] Read receipts: DM shows "Read" timestamp, channel shows count
- [ ] Presence: green dot when online, gray when offline
- [ ] No plaintext media keys or URLs in database

---

## S6 — Live Canvas + Explore Page (Weeks 13–14)

**Goal:** Server-side Live Canvas panel (music, polls, clips carousel) + Explore/Discovery page.

### Week 13: Live Canvas (Phase 7)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migrations: `canvas_music_state`, `polls`, `poll_votes` tables | BE | [ ] |
| 2 | Create `src/canvas/`: handlers + service | BE | [ ] |
| 3 | Implement `PATCH /api/v1/servers/{id}/canvas/music`: set music state + WS broadcast | BE | [ ] |
| 4 | Implement `POST /api/v1/channels/{id}/polls`: create poll | BE | [ ] |
| 5 | Implement `POST /api/v1/polls/{id}/vote`: submit vote, prevent double-voting (409) | BE | [ ] |
| 6 | Implement live vote count via WS `poll_vote` event (incremental update) | BE | [ ] |
| 7 | Implement `GET /api/v1/servers/{id}/clips`: recent Clip metadata | BE | [ ] |
| 8 | Build `LiveCanvas.svelte`: right-side panel container | FE | [ ] |
| 9 | Build `MusicWidget.svelte`: album art, artist, title, animated pulse | FE | [ ] |
| 10 | Build `PollWidget.svelte`: options with live bar fill animation | FE | [ ] |
| 11 | Build `ClipsCarousel.svelte`: horizontal scroll of Clip thumbnails | FE | [ ] |
| 12 | Write tests: music state broadcast, poll vote dedup, clips query | BE | [ ] |

### Week 14: Explore / Discovery (Phase 8)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Create `src/servers/explore.rs` | BE | [ ] |
| 2 | Implement `GET /api/v1/explore/trending-tags`: GROUP BY + cached 5 min | BE | [ ] |
| 3 | Implement `GET /api/v1/explore/live-servers`: active in last 30 min | BE | [ ] |
| 4 | Implement `GET /api/v1/explore/communities`: public servers by member count | BE | [ ] |
| 5 | Implement `GET /api/v1/explore/top-yappers`: by follower count | BE | [ ] |
| 6 | Implement `GET /api/v1/search?q=`: full-text search (pg_trgm) | BE | [ ] |
| 7 | Ensure `pg_trgm` extension + GIN index created in migration | BE | [ ] |
| 8 | Build Explore page: `(app)/explore/+page.svelte` with grid/list toggle | FE | [ ] |
| 9 | Build `TrendingTags.svelte`: horizontal chip row | FE | [ ] |
| 10 | Build `LiveServerCard.svelte`: gradient bg, member count, pulse indicator | FE | [ ] |
| 11 | Build `CommunityCard.svelte`: server card with description + join button | FE | [ ] |
| 12 | Build search bar with debounced query | FE | [ ] |
| 13 | Write tests: search results, trending cache, live server detection | BE | [ ] |

### S6 Acceptance Criteria
- [ ] Admin sets music state → all connected canvases update in real time
- [ ] Poll created → vote → bar fills live → second vote returns 409
- [ ] Clips carousel shows recent Clip thumbnails
- [ ] Explore page renders trending tags, live servers, communities
- [ ] Search returns matching servers and users via pg_trgm
- [ ] Trending tags cached (verify cache invalidation after 5 min)

---

## S7 — Profiles + Parental Controls (Weeks 15–16)

**Goal:** Public user profiles with social graph + full parental dashboard with approval workflows.

### Week 15: User Profile & Social Graph (Phase 9)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migrations: `friendships`, `followers`, `hype_moments` tables + indexes | BE | [ ] |
| 2 | Implement `GET /api/v1/users/{username}`: public profile with counts + mutual followers | BE | [ ] |
| 3 | Implement `POST /api/v1/users/{username}/follow`, `DELETE .../follow` | BE | [ ] |
| 4 | Implement `GET /api/v1/users/me/feed`: activity feed from followed users | BE | [ ] |
| 5 | Implement `POST /api/v1/hype-moments`: pin message to profile | BE | [ ] |
| 6 | Implement `GET /api/v1/users/{username}/hype-moments` | BE | [ ] |
| 7 | Build Profile page: `(app)/profile/[username]/+page.svelte` | FE | [ ] |
| 8 | Build `ProfileHeader.svelte`: avatar, banner, name, @username, location, follower counts | FE | [ ] |
| 9 | Build `HypeMoments.svelte`: masonry grid (Yap card, Clip card, text card) | FE | [ ] |
| 10 | Build `MutualConnections.svelte`: avatar row of mutual followers | FE | [ ] |
| 11 | Build "Top Communities" server chip row | FE | [ ] |
| 12 | Write tests: follow/unfollow, profile data, hype moment pinning | BE | [ ] |

### Week 16: Parental Controls (Phase 10)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migrations: `parent_child_relationships`, `pending_friend_requests`, `pending_server_joins`, `parent_notifications`, `parental_action_audit` + indexes | BE | [ ] |
| 2 | Create `src/parental/`: handlers, service, approval logic | BE | [ ] |
| 3 | Implement child account creation: `POST /api/v1/parental/children` with COPPA consent flow | BE | [ ] |
| 4 | Implement friend request interception: if child has `parental_controls_enabled` → insert `pending_friend_requests` → push to parent | BE | [ ] |
| 5 | Implement server join interception: → insert `pending_server_joins` → push to parent | BE | [ ] |
| 6 | Implement `PATCH /api/v1/parental/friend-requests/{id}/approve|decline` | BE | [ ] |
| 7 | Implement `PATCH /api/v1/parental/server-joins/{id}/approve|decline` | BE | [ ] |
| 8 | Implement `GET /api/v1/parental/children/{id}/overview`, `/safety-feed`, `/notifications` | BE | [ ] |
| 9 | Extend WS hub to send `parent_notification` events to parent's connected session | BE | [ ] |
| 10 | Build child setup wizard (Screens 5 & 6): 3-step flow with Safety Gates toggles | FE | [ ] |
| 11 | Build `SafetyDashboard.svelte`: managed children sidebar + center panel | FE | [ ] |
| 12 | Build `PendingAlerts.svelte`: friend request cards + server join cards (approve/decline) | FE | [ ] |
| 13 | Build `SafetyFeed.svelte`: timeline of metadata events | FE | [ ] |
| 14 | Build `ActivitySnapshot.svelte`: top communities, most interacted | FE | [ ] |
| 15 | Write E2E test: friend request to child → parent notification → approve → friendship created | BE/FE | [ ] |
| 16 | Write E2E test: child server join → pending → parent approves → child in server | BE/FE | [ ] |

### S7 Acceptance Criteria
- [ ] Profile page renders with correct follower/following counts and Hype Moments
- [ ] Follow/unfollow works, mutual connections displayed
- [ ] Parent creates child account with COPPA consent (DOB < 13)
- [ ] Friend request to child → parent receives real-time WS notification
- [ ] Parent approves friend request → friendship created, child sees friend
- [ ] Child attempts server join → pending → parent approves → child added
- [ ] Safety Gates toggles persist and are enforced server-side
- [ ] Audit trail: all parental actions logged in `parental_action_audit`

---

## S8 — Screen Time + Discord Integration (Weeks 17–18)

**Goal:** OS-level screen time reporting + Discord profile import + bot migration tool.

### Week 17: Screen Time (Phase 11)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migration: `screen_time_records` table | BE | [ ] |
| 2 | Implement `POST /api/v1/screentime/report`: accept batched records | BE | [ ] |
| 3 | Implement `GET /api/v1/parental/children/{id}/screentime?period=week`: aggregated data | BE | [ ] |
| 4 | Create `frontend/ios/App/App/plugins/ScreenTimePlugin.swift`: FamilyControls + DeviceActivityMonitor | FE | [ ] |
| 5 | Create `frontend/android/app/src/main/java/plugins/ScreenTimePlugin.kt`: UsageStatsManager | FE | [ ] |
| 6 | Create `frontend/src/lib/plugins/screentime.ts`: Capacitor plugin JS bridge | FE | [ ] |
| 7 | Implement in-app session tracking: `session_start` on foreground, `session_end` on blur | FE | [ ] |
| 8 | Add screen time chart to parental dashboard `ActivitySnapshot.svelte` | FE | [ ] |
| 9 | Write tests: report ingestion, aggregation query, permission flow | BE | [ ] |

### Week 18: Discord Integration (Phase 12)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migrations: `bot_applications`, `bot_tokens` tables | BE | [ ] |
| 2 | Create `src/discord/importer.rs`: Discord API client for profile fetch | BE | [ ] |
| 3 | Implement Discord profile import: OAuth2 → fetch profile → download avatar → re-upload to R2 → pre-fill form | BE | [ ] |
| 4 | Store `discord_id` in users table for re-link detection | BE | [ ] |
| 5 | Create `src/discord/bot_importer.rs`: bot token exchange + app info fetch | BE | [ ] |
| 6 | Create `src/bots/`: handlers, service, bot auth middleware (`Authorization: Bot {token}`) | BE | [ ] |
| 7 | Implement `POST /api/v1/bots/import-discord`: fetch Discord bot info → create Yapper bot account → generate token | BE | [ ] |
| 8 | Build `DiscordImport.svelte`: "Import from Discord" button in onboarding + settings | FE | [ ] |
| 9 | Build `DeveloperPortal.svelte`: bot management section in settings | FE | [ ] |
| 10 | Build `BotMigrationTool.svelte`: Discord token input → migration guide display | FE | [ ] |
| 11 | Write tests: OAuth flow, avatar R2 re-upload, bot token generation + auth | BE | [ ] |

### S8 Acceptance Criteria
- [ ] iOS ScreenTime plugin requests authorization + reports data to API
- [ ] Android UsageStats plugin requests permission + reports data to API
- [ ] Screen time data visible in parental dashboard with weekly aggregation
- [ ] Discord OAuth → profile pre-filled correctly, avatar stored in R2 (not Discord CDN)
- [ ] Bot import: Discord token → Yapper bot account created → token displayed once
- [ ] Bot token authenticates and can POST message to test channel
- [ ] In-app session time tracked even without OS-level permission

---

## S9 — Emojis + Settings (Weeks 19–20)

**Goal:** Server-scoped custom emoji system + full user settings panel.

### Week 19: Custom Emoji System (Phase 13)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migration: `server_emojis` table with `UNIQUE(server_id, name)` | BE | [ ] |
| 2 | Create `src/emojis/`: handlers, service, `processor.rs` (WebP via `image` crate) | BE | [ ] |
| 3 | Implement `POST /api/v1/servers/{id}/emojis`: validate, convert PNG→WebP 64x64, upload R2, broadcast `emoji_added` WS event | BE | [ ] |
| 4 | Implement `GET /api/v1/servers/{id}/emojis`: emoji list (cacheable) | BE | [ ] |
| 5 | Implement `DELETE /api/v1/servers/{id}/emojis/{emoji_id}`: admin-only, broadcast `emoji_removed` | BE | [ ] |
| 6 | Enforce limit: 50 per server (100 for premium) | BE | [ ] |
| 7 | Build `EmojiPicker.svelte`: tabs for Unicode + server custom emojis, search | FE | [ ] |
| 8 | Build `EmojiUploader.svelte`: drag-and-drop upload with preview | FE | [ ] |
| 9 | Build `CustomEmojiManager.svelte`: admin management panel (list, delete) | FE | [ ] |
| 10 | Integrate emoji picker trigger in `MessageInput.svelte` | FE | [ ] |
| 11 | Implement `:emoji_name:` parsing in message renderer → `<img>` tag | FE | [ ] |
| 12 | Cache emoji list in IndexedDB, invalidate on WS events | FE | [ ] |
| 13 | Write tests: upload → WebP conversion, 403 for non-admin, limit enforcement, shortcode rendering | BE/FE | [ ] |

### Week 20: User Settings (Phase 14)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Write migration: user settings columns / table | BE | [ ] |
| 2 | Implement profile update endpoints: avatar, banner, display name, about me, theme color, location | BE | [ ] |
| 3 | Implement username change with 30-day cooldown | BE | [ ] |
| 4 | Implement privacy settings: DM controls, search visibility, block list | BE | [ ] |
| 5 | Implement `GET /api/v1/account/data-export`: GDPR ZIP (profile JSON, friend list, server list, message metadata) | BE | [ ] |
| 6 | Implement `DELETE /account`: 30-day soft delete with async PII purge job | BE | [ ] |
| 7 | Build Settings page: `(app)/settings/+page.svelte` with sidebar navigation | FE | [ ] |
| 8 | Build `ProfileForm.svelte`: avatar/banner upload, all profile fields | FE | [ ] |
| 9 | Build `ThemeColorPicker.svelte`: 8 presets + custom hex | FE | [ ] |
| 10 | Build appearance settings: dark/light/auto, font size, message density | FE | [ ] |
| 11 | Build notification toggles per notification type | FE | [ ] |
| 12 | Build `DangerZone.svelte`: disable account, delete account (confirmation modal) | FE | [ ] |
| 13 | Write tests: profile update, username cooldown, data export content, soft delete | BE | [ ] |

### S9 Acceptance Criteria
- [ ] Admin uploads PNG → R2 stores WebP at 64x64
- [ ] `:custom_name:` in message → renders as emoji image for all channel members
- [ ] Non-admin upload → 403; 51st emoji → 400 (limit enforced)
- [ ] Emoji picker shows Unicode + custom tabs with search
- [ ] Settings page: all sections functional (profile, privacy, appearance, notifications, danger zone)
- [ ] Username change enforces 30-day cooldown
- [ ] Data export: ZIP contains profile JSON, no plaintext message content
- [ ] Account deletion: soft delete → user cannot log in

---

## S10 — Desktop Polish + Security Audit (Weeks 21–22)

**Goal:** Native desktop feel via Tauri + comprehensive security hardening pass.

### Week 21: Tauri Desktop Polish (Phase 15)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Implement system tray: Tauri v2 tray plugin, minimize to tray, unread badge | FE | [ ] |
| 2 | Implement native notifications: Tauri notification plugin | FE | [ ] |
| 3 | Implement secure key storage: Tauri `stronghold` plugin for Signal keys | FE | [ ] |
| 4 | Implement auto-updater: Tauri updater plugin with update manifest | FE | [ ] |
| 5 | Implement keyboard shortcuts: `Ctrl+K` search, `Ctrl+,` settings | FE | [ ] |
| 6 | Implement custom title bar: hide OS title bar, SvelteKit custom bar | FE | [ ] |
| 7 | Implement deep links: `yapper://` protocol registration | FE | [ ] |
| 8 | Create `src/lib/plugins/tauri-compat.ts`: unified interface for Tauri + Capacitor | FE | [ ] |
| 9 | Configure Windows NSIS installer | FS | [ ] |
| 10 | Configure macOS DMG + .app bundle | FS | [ ] |
| 11 | Configure Linux AppImage + `.deb` | FS | [ ] |
| 12 | Test on all 3 desktop platforms | FE/FS | [ ] |

### Week 22: Security Audit & GDPR (Phase 16)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Audit all rate limits: verify `governor` config per route (5/sec messages, 100/min API) | BE | [ ] |
| 2 | Audit security headers: CSP, HSTS, X-Frame-Options, nosniff — verify in production | BE | [ ] |
| 3 | Audit CORS: verify strict allowlist (`yapperhq.com`, `tauri://localhost`, `capacitor://localhost`) | BE | [ ] |
| 4 | Audit input validation: verify `#[serde(deny_unknown_fields)]` on ALL input DTOs | BE | [ ] |
| 5 | Audit CSRF: verify `SameSite=Strict` + `X-CSRF-Token` on all state-changing endpoints | BE | [ ] |
| 6 | Audit WebSocket: verify first-message auth, max frame 64KB, max 5 connections/user | BE | [ ] |
| 7 | Audit password handling: verify Argon2id params (`m=65536, t=3, p=4`), reject > 1KB | BE | [ ] |
| 8 | Audit JWT: verify RS256 + `kid` rotation, 15-min TTL | BE | [ ] |
| 9 | Implement safety numbers: SHA-256 fingerprint display of identity keys + change alert | FE | [ ] |
| 10 | Verify GDPR data export: no plaintext in ZIP | BE | [ ] |
| 11 | Verify right to erasure: soft delete → PII purge job → username anonymized to `[deleted]` | BE | [ ] |
| 12 | Verify COPPA: age gate, consent flow, no behavioral analytics for child accounts | BE | [ ] |
| 13 | Run `cargo audit` + `npm audit` — fix any vulnerabilities | FS | [ ] |
| 14 | Run Playwright security smoke tests: XSS attempts, injection attempts | FE | [ ] |
| 15 | Document all security findings in `SECURITY_AUDIT.md` | BE | [ ] |

### S10 Acceptance Criteria
- [ ] Tauri desktop: system tray, native notifications, keyboard shortcuts, custom title bar — all functional
- [ ] Auto-updater checks for updates on launch
- [ ] Deep links: `yapper://invite/CODE` opens app and processes invite
- [ ] All 3 desktop installers build successfully
- [ ] Security audit: all items pass or have documented mitigations
- [ ] `cargo audit` + `npm audit`: zero high/critical vulnerabilities
- [ ] GDPR: data export works, erasure job runs, COPPA consent enforced
- [ ] `SECURITY_AUDIT.md` completed and reviewed

---

## S11 — Premium Prep + Launch (Weeks 23–24)

**Goal:** Premium feature flags, monitoring, final E2E testing, app store submissions, go live.

### Week 23: Premium + Monitoring (Phase 17)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Add `is_premium` column to users table (migration) | BE | [ ] |
| 2 | Implement premium gating: 100 emojis (vs 50), larger uploads (50MB vs 10MB), custom badge | BE | [ ] |
| 3 | Build `PremiumPlaceholder.svelte`: feature comparison table (Free vs GoPro) | FE | [ ] |
| 4 | Build GoPro banner in settings sidebar (gradient card, "Coming soon") | FE | [ ] |
| 5 | Build premium lock badge on gated features → "Coming soon" modal | FE | [ ] |
| 6 | Integrate Sentry: `sentry-rust` in backend, `@sentry/sveltekit` in frontend | FS | [ ] |
| 7 | Verify Cloudflare Analytics is active on Pages | FS | [ ] |
| 8 | Verify Fly.io metrics dashboard accessible (CPU, memory, requests) | FS | [ ] |
| 9 | Set up GitHub Actions: `fly deploy` on push to `main` | FS | [ ] |
| 10 | Configure all production secrets in Fly.io (`fly secrets set ...`) | FS | [ ] |

### Week 24: Launch Preparation (Phase 18)

| # | Task | Owner | Done |
|---|------|-------|------|
| 1 | Run full E2E testing checklist (all 13 scenarios from plan) | ALL | [ ] |
| 2 | Test: new user registers → onboards → sees Explore page | FE | [ ] |
| 3 | Test: two users exchange E2EE DMs → verify DB has no plaintext | BE | [ ] |
| 4 | Test: Audio Yap sent → received and decrypted | FE | [ ] |
| 5 | Test: parent creates child (DOB<13) → COPPA consent email | BE | [ ] |
| 6 | Test: friend request to child → parent notification → approve → friendship | BE/FE | [ ] |
| 7 | Test: child server join → pending → approve → membership | BE/FE | [ ] |
| 8 | Test: Screen Time data from mobile → appears in dashboard | FE | [ ] |
| 9 | Test: Discord import → avatar in R2 | BE | [ ] |
| 10 | Test: bot import → bot sends message | BE | [ ] |
| 11 | Test: custom emoji → renders in messages | FE | [ ] |
| 12 | Test: data export → no plaintext; account deletion → PII purge | BE | [ ] |
| 13 | Deploy final production build to Fly.io | FS | [ ] |
| 14 | Deploy frontend to Cloudflare Pages (production) | FS | [ ] |
| 15 | Configure custom domains: `api.yapperhq.com`, `app.yapperhq.com` | FS | [ ] |
| 16 | Submit to Apple App Store (requires Mac + Xcode + signing) | FS | [ ] |
| 17 | Submit to Google Play Store | FS | [ ] |
| 18 | Update marketing site: remove "Coming soon", add download links | FE | [ ] |
| 19 | Send launch notification to wishlist (Resend bulk via Worker) | FS | [ ] |

### S11 Acceptance Criteria
- [ ] Premium gating works: free users see lock badges, premium flags respected server-side
- [ ] Sentry captures errors in both backend and frontend
- [ ] All 13 E2E testing checklist items pass
- [ ] Production backend accessible at `api.yapperhq.com`
- [ ] Production frontend accessible at `app.yapperhq.com`
- [ ] iOS app submitted to App Store (or TestFlight)
- [ ] Android app submitted to Google Play
- [ ] Marketing site updated with download links
- [ ] Wishlist notification email sent

---

## Sprint Ceremonies

### Per Sprint
| Ceremony | When | Duration | Purpose |
|----------|------|----------|---------|
| Sprint Planning | Day 1 (Monday) | 1 hour | Assign tasks, clarify acceptance criteria |
| Daily Standup | Every morning | 15 min | Blockers, progress, plan for today |
| Mid-Sprint Check | End of Week 1 | 30 min | Course-correct if behind |
| Sprint Review | Last day | 1 hour | Demo deliverables, verify acceptance criteria |
| Retrospective | Last day | 30 min | What worked, what didn't, action items |

### Definition of Done (per task)
1. Code written and compiles without warnings
2. Tests written and passing (unit + integration where applicable)
3. CI pipeline green
4. Code reviewed (if team > 1)
5. No known security vulnerabilities introduced
6. Acceptance criteria for the task met

### Definition of Done (per sprint)
1. ALL sprint acceptance criteria checked off
2. CI pipeline green on `main`
3. No P0/P1 bugs remaining
4. Documentation updated (if applicable)
5. Demo completed successfully

---

## Dependency Graph (Critical Path)

```
S0 ──→ S1 ──→ S2 ──→ S3 ──→ S4 ──→ S5 ──→ S6 ──→ S7 ──→ S8 ──→ S9 ──→ S10 ──→ S11
                                                    │
S0 (marketing) runs in parallel, no dependencies ───┘
```

**Critical path:** S1 → S2 → S3 → S4 → S5 (everything after S5 can be partially parallelized by a 2+ person team)

**Parallelization opportunities (2+ person team):**
- S6: Canvas (BE) can parallel with Explore (FE) — different endpoints, different pages
- S7: Profile (backend-heavy) can parallel with Parental dashboard (frontend-heavy)
- S8: Screen Time (native plugins) can parallel with Discord Integration (API work)
- S9: Emojis (standalone feature) can parallel with Settings (standalone feature)
- S10: Tauri (frontend) can parallel with Security Audit (backend)

---

## Risk Register (Sprint-Level)

| Sprint | Risk | Impact | Mitigation |
|--------|------|--------|------------|
| S0 | Apple FamilyControls entitlement delay | Blocks S8 Screen Time iOS | Apply Day 1; soft-launch without iOS Screen Time |
| S3 | libsignal WASM integration complexity | Delays E2EE | Budget extra time; Signal's Rust crate has good docs |
| S4 | Sender Key distribution at scale | Performance issues | Cap MVP servers at 500 members |
| S5 | MediaRecorder API inconsistencies across browsers | Broken recording | Test on Chrome, Firefox, Safari; polyfill as needed |
| S7 | COPPA consent flow edge cases | Legal compliance risk | Test thoroughly; add idempotency checks |
| S8 | Discord API rate limits during import | Failed imports | Implement retry with backoff |
| S10 | Security audit findings requiring rework | Schedule slip | Budget 2-3 days buffer; most security was built in from S1 |
| S11 | App Store review rejection | Launch delay | Submit early for review; have TestFlight/beta track ready |

---

## Quick Reference: Environment Variables

All sprints reference these. Store in `.env.local` for dev, `fly secrets set` for production.

```env
DATABASE_URL=postgresql://...          # Neon connection string
JWT_PRIVATE_KEY=...                    # RS256 private key (PEM)
JWT_PUBLIC_KEY=...                     # RS256 public key (PEM)
R2_ACCOUNT_ID=...                      # Cloudflare account ID
R2_ACCESS_KEY_ID=...                   # R2 API token
R2_SECRET_ACCESS_KEY=...               # R2 API secret
R2_BUCKET_NAME=yapper-media
RESEND_API_KEY=...                     # Transactional email
DISCORD_CLIENT_ID=...                  # OAuth2
DISCORD_CLIENT_SECRET=...
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
APPLE_CLIENT_ID=...
APPLE_TEAM_ID=...
APPLE_KEY_ID=...
APPLE_PRIVATE_KEY=...
FCM_SERVICE_ACCOUNT_JSON=...           # Firebase Cloud Messaging
SENTRY_DSN=...                         # Error monitoring
FRONTEND_URL=https://app.yapperhq.com    # CORS origin
```
