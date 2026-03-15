# YAPPER — Developer Handover Document

**Last updated:** 2026-03-16 (rev 3)
**Project status:** Active development — S0–S12 complete; Support tickets + HubSpot integration live; CI build pipeline optimised (GHA layer cache, ~90s deploys)
**Full implementation plan:** `C:\Users\rajma\.claude\plans\quizzical-yawning-starfish.md`

---

## 1. What Is Yapper?

Yapper is a greenfield SaaS real-time chat platform targeting community/social audiences. Think Discord-like server/channel UX, but with:

- **End-to-end encryption (E2EE)** via Signal Protocol on all user messages
- **Parental safety controls** (COPPA-compliant, metadata-only — server never sees plaintext)
- **Voice "Yaps"** (audio messages) and **Video "Clips"** (video messages), both E2EE
- **Live Canvas** sidebar (music state, polls, community clips carousel)
- **Discord profile import** + bot migration tool
- **Custom server-scoped emojis**

### The Inviolable Rule

> **E2EE is non-negotiable.** The server never holds decrypted message content under any circumstance. Parental controls operate exclusively on metadata (friend requests, server joins, screen time). Any design that would require the server to read message content is rejected.

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────┐
│                     CLIENTS                               │
│  SvelteKit (Web PWA) │ Tauri v2 (Desktop) │ Capacitor (Mobile) │
│  libsignal WASM      │ libsignal WASM     │ libsignal WASM     │
└───────────┬──────────┴─────────┬──────────┴────────┬─────┘
            │ HTTPS + WSS                             │
            ▼                                         ▼
┌──────────────────────────────────────────────────────────┐
│              RUST BACKEND (single binary)                 │
│  Axum HTTP Router + WebSocket Hub                        │
│  ┌─────────┐ ┌──────────┐ ┌────────────┐ ┌───────────┐  │
│  │ REST API│ │ WS Hub   │ │ Signal Keys│ │ Media/R2  │  │
│  │ (Axum)  │ │(DashMap) │ │ (native)   │ │ (presign) │  │
│  └────┬────┘ └────┬─────┘ └─────┬──────┘ └─────┬─────┘  │
│       └───────────┴─────────────┴───────────────┘        │
│                         │                                 │
│  Fly.io (shared-cpu-1x, 256MB)                           │
└─────────────────────────┬────────────────────────────────┘
                          │
         ┌────────────────┼────────────────┐
         ▼                ▼                ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  Neon        │  │ Cloudflare   │  │ Cloudflare   │
│  PostgreSQL  │  │ R2 (media)   │  │ Pages (CDN)  │
│  (0.5GB free)│  │ (10GB free)  │  │ (unlimited)  │
└──────────────┘  └──────────────┘  └──────────────┘
```

### Key Technology Choices

| Layer | Technology | Why |
|-------|-----------|-----|
| Backend language | **Rust** | Already required for Tauri; `libsignal-protocol` is native Rust; ~10MB idle RAM fits Fly.io 256MB VM; `sqlx::query!` compile-time SQL verification |
| Backend framework | **Axum + Tokio** | Tower-based middleware, native WebSocket upgrade, async/await |
| Frontend framework | **SvelteKit** | Static adapter for Tauri/Capacitor; reactive stores; small bundle size |
| Desktop shell | **Tauri v2** | Rust-native; ~5MB binary vs Electron's ~150MB |
| Mobile bridge | **Capacitor** | WebView-based; native plugins for iOS/Android APIs |
| Database | **Neon PostgreSQL** | Serverless free tier; ACID for Signal keys; LISTEN/NOTIFY replaces Redis pub/sub |
| Object storage | **Cloudflare R2** | S3-compatible; 10GB free; no egress fees |
| Backend hosting | **Fly.io** | Always-on free VMs (no cold starts like Render) |
| Frontend hosting | **Cloudflare Pages** | Unlimited bandwidth; global CDN; auto-deploy on push |
| E2EE | **Signal Protocol** | X3DH + Double Ratchet (DMs), Sender Keys (groups); gold standard |
| Email | **Resend** | 3K/month free; transactional only |

### What Is NOT in the Stack

- **No Redis** — in-memory Rust `DashMap` for ephemeral state; PostgreSQL for everything persistent
- **No SpaceTimeDB** — evaluated, deferred to post-MVP (1GB limit, compliance needs PostgreSQL anyway)
- **No Stripe** — premium tier is placeholder UI only for MVP
- **No content moderation** — removed from MVP; E2EE makes server-side moderation impossible by design

---

## 3. Coding Standards

This project follows coding standards derived from **NASA/JPL's "Power of Ten" rules** (Gerard Holzmann, 2006), adapted for Rust and TypeScript. These rules exist because Yapper handles E2EE keys, COPPA-protected child data, and financial-grade auth tokens.

### Rule 1: Simple Control Flow

**NASA original:** Restrict all code to very simple control flow constructs. Do not use `goto`, `setjmp/longjmp`, or recursion.

**Yapper application:**
- No unbounded recursion. Any recursive algorithm must have a provable depth bound or use an iterative equivalent. If recursion is necessary, add a `max_depth` parameter with a hard limit.
- Prefer `match` over deeply nested `if/else` chains in Rust.
- Prefer early returns (`guard clauses`) over deep nesting.
- Maximum nesting depth: **4 levels**. If you hit 5, extract a function.

```rust
// BAD: deep nesting
fn process(msg: &Message) {
    if msg.is_valid() {
        if let Some(channel) = get_channel(msg.channel_id) {
            if channel.is_active() {
                if let Ok(member) = check_membership(msg.sender_id, channel.id) {
                    // actual logic buried 4 levels deep
                }
            }
        }
    }
}

// GOOD: guard clauses
fn process(msg: &Message) -> Result<()> {
    if !msg.is_valid() {
        return Err(AppError::InvalidMessage);
    }
    let channel = get_channel(msg.channel_id)?;
    if !channel.is_active() {
        return Err(AppError::ChannelInactive);
    }
    let member = check_membership(msg.sender_id, channel.id)?;
    // logic at 0 nesting
    Ok(())
}
```

### Rule 2: Fixed Upper Bounds on Loops

**NASA original:** All loops must have a fixed upper bound. It must be trivially possible for a checking tool to prove statically that a preset upper bound on the number of iterations of a loop cannot be exceeded.

**Yapper application:**
- Every `while` loop and `loop` must have an explicit iteration cap or a guaranteed termination condition.
- WebSocket message processing loops must have a `max_messages_per_tick` constant.
- Retry loops must use bounded backoff with a maximum attempt count (never infinite retry).
- Prefer `for item in collection.iter().take(LIMIT)` over open-ended iteration.

```rust
// BAD: unbounded retry
loop {
    match try_connect().await {
        Ok(conn) => return conn,
        Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
    }
}

// GOOD: bounded retry
const MAX_RETRIES: u32 = 5;
for attempt in 0..MAX_RETRIES {
    match try_connect().await {
        Ok(conn) => return Ok(conn),
        Err(e) if attempt == MAX_RETRIES - 1 => return Err(e),
        Err(_) => {
            let backoff = Duration::from_millis(100 * 2u64.pow(attempt));
            tokio::time::sleep(backoff).await;
        }
    }
}
```

### Rule 3: No Dynamic Memory Allocation After Initialization

**NASA original:** Do not use dynamic memory allocation after initialization.

**Yapper adaptation (relaxed for application software):**
- Pre-allocate collections with known capacities: `Vec::with_capacity(n)`, `HashMap::with_capacity(n)`.
- Set explicit capacity limits on all unbounded collections (the hub's `DashMap`, message buffers, etc.).
- Define `const MAX_*` constants for all buffer sizes — never let a client-controlled value determine allocation size without a cap.
- WebSocket frames: **max 64KB**. Reject larger frames immediately.
- Upload sizes: **max 10MB** (50MB for premium). Enforce server-side, not just client-side.
- OPK batch upload: **max 100 keys per request**.
- Emoji upload: **max 256KB** per image.

```rust
// Constants file (src/constants.rs)
pub const MAX_WS_FRAME_SIZE: usize = 64 * 1024;        // 64KB
pub const MAX_UPLOAD_SIZE: usize = 10 * 1024 * 1024;    // 10MB
pub const MAX_UPLOAD_SIZE_PREMIUM: usize = 50 * 1024 * 1024;
pub const MAX_OPK_BATCH: usize = 100;
pub const MAX_EMOJI_SIZE: usize = 256 * 1024;           // 256KB
pub const MAX_CONNECTIONS_PER_USER: usize = 5;
pub const MAX_SERVER_MEMBERS_MVP: usize = 500;
pub const MAX_MESSAGE_LENGTH: usize = 4000;              // characters
pub const MAX_PASSWORD_LENGTH: usize = 1024;             // reject > 1KB
```

### Rule 4: No Function Longer Than 60 Lines

**NASA original:** No function should be longer than what can be printed on a single sheet of paper (approximately 60 lines).

**Yapper application:**
- Maximum function body: **60 lines** of logic (excluding blank lines and comments).
- Axum handler functions may exceed this if they are primarily chaining validations — but extract the core logic into a `service.rs` function.
- Keep `handlers.rs` thin (parse request → call service → format response). Business logic lives in `service.rs`.

```
handlers.rs  → HTTP concerns (extract params, call service, return Response)
service.rs   → Business logic (validate, query DB, compute)
mod.rs       → Router assembly (pub mod handlers; pub mod service;)
```

### Rule 5: Minimum Two Assertions Per Function

**NASA original:** The assertion density of the code should average to a minimum of two assertions per function.

**Yapper adaptation:**
- Every `service.rs` function must validate its preconditions before proceeding. Use `debug_assert!` for invariants and `Result<T, AppError>` for expected failures.
- Every handler must validate input before passing to service layer.
- **Never trust client input** — validate at the boundary, assert at the core.

```rust
// service.rs
pub async fn create_server(user_id: Uuid, input: CreateServer, pool: &PgPool) -> Result<Server> {
    // Precondition assertions
    debug_assert!(!input.name.is_empty(), "name should be validated by handler");
    assert!(input.name.len() <= 100, "server name exceeds maximum length");

    // Business logic...
    let slug = slugify(&input.name);
    let server = sqlx::query_as!(Server, "INSERT INTO servers ...")
        .fetch_one(pool)
        .await?;

    // Postcondition
    debug_assert!(server.owner_id == user_id);
    Ok(server)
}
```

### Rule 6: Declare Variables at Smallest Scope

**NASA original:** Data objects must be declared at the smallest possible level of scope.

**Yapper application:**
- No global mutable state except the explicitly designed shared structures (`DashMap` hub, `PgPool`).
- Shared state is always behind `Arc<T>` and passed via Axum's `State` extractor — never via static mutable globals.
- Prefer `let` bindings inside the block where they're used. Don't declare variables "just in case" at the top of a function.
- Svelte stores: scope stores to the narrowest component tree that needs them. Don't put everything in a global store.

### Rule 7: Check Return Values

**NASA original:** The return value of non-void functions must be checked by each calling function, or explicitly cast to `void`.

**Yapper application:**
- In Rust, the `Result` type enforces this via `#[must_use]`. Never use `let _ = fallible_call()` without a comment explaining why the error is intentionally ignored.
- In TypeScript, never use `void` on a Promise without `.catch()` or `try/catch`. Unhandled rejections crash the app.
- All database queries must handle the error case — never `unwrap()` a query result in production code. Use `?` operator.
- `unwrap()` is permitted only in tests and in `main.rs` setup (where panicking is the correct behavior for misconfiguration).

```rust
// BAD
let _ = sqlx::query!("DELETE FROM sessions WHERE expired < now()")
    .execute(&pool)
    .await;

// GOOD
sqlx::query!("DELETE FROM sessions WHERE expired < now()")
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to clean expired sessions: {e}");
        e
    })?;
```

### Rule 8: Limited Preprocessor / Macro Use

**NASA original:** The use of the preprocessor must be limited to the inclusion of header files and simple conditional macros.

**Yapper application:**
- Limit Rust macro usage to: `derive` macros, `sqlx::query!`/`query_as!`, `tracing::instrument`, `serde` attributes, and `cfg(test)`.
- Do not write custom procedural macros unless they eliminate a proven, repetitive pattern across 5+ call sites.
- `cfg` attributes are acceptable for platform-specific code (Tauri vs Capacitor plugin bridges).
- In TypeScript, avoid template literal types for business logic — they're hard to debug.

### Rule 9: Restrict Pointer / Reference Use

**NASA original:** The use of pointers should be restricted. No more than one level of dereferencing.

**Yapper adaptation:**
- Limit `Arc<Mutex<Arc<...>>>` nesting. Maximum two levels of smart pointer wrapping. The hub uses `Arc<DashMap<K, V>>` (one level) — this is the upper bound for normal code.
- Prefer cloning over complex lifetime annotations when the data is small (<1KB). Correctness over micro-optimization.
- No `unsafe` blocks unless wrapping a C FFI call (e.g., if `libsignal-protocol` requires it). Every `unsafe` block must have a `// SAFETY:` comment explaining the invariant.
- In TypeScript, avoid deeply nested optional chaining (`a?.b?.c?.d?.e`) — extract intermediate variables.

### Rule 10: Compile with All Warnings Enabled

**NASA original:** All code must be compiled with all compiler warnings enabled at the highest warning level. All code must compile without warnings.

**Yapper application:**
- Rust: `#![deny(warnings)]` in `main.rs`. CI runs `cargo clippy -- -D warnings`.
- TypeScript: `"strict": true` in `tsconfig.json`. No `@ts-ignore` without a linked issue number.
- CI gate: code does not merge if `cargo clippy`, `cargo fmt --check`, `npm run lint`, or `npm run check` fail.
- `cargo audit` and `npm audit` run in CI — known vulnerabilities block merge.

```toml
# .cargo/config.toml
[build]
rustflags = ["-D", "warnings"]
```

---

## 4. Security Standards

These are non-negotiable security requirements, not suggestions.

### Authentication
- **Passwords:** Argon2id (`m=65536, t=3, p=4`). Hard reject > 1KB (400 error, not truncation).
- **JWT:** RS256 with `kid` header for key rotation. 15-minute access token TTL. Private key never leaves API service environment.
- **Refresh tokens:** HttpOnly, Secure, SameSite=Strict cookie. Family-based reuse detection — replaying a revoked token invalidates the entire family.
- **CSRF:** Double-submit token (`X-CSRF-Token` header) validated on all state-changing endpoints.
- **Rate limiting:** 5 failed logins → 15-minute IP lockout. In-memory `governor` crate, per-IP + per-user.

### WebSocket
- **Auth via first message** — client sends `{ type: "auth", token: "..." }` as the first frame. Token is NEVER in the query string (avoids log leakage).
- **Re-auth:** Server sends `{ type: "re_auth_required" }` 60 seconds before JWT expiry. Client must respond with fresh token or connection drops.
- **Limits:** Max frame size 64KB. Max 5 connections per user.

### E2EE
- Signal Protocol: X3DH + Double Ratchet (DMs), Sender Keys (groups).
- **No server-side column for media keys.** R2 object key + AES key + IV are embedded inside the Signal ciphertext. The server cannot correlate R2 objects to messages.
- **Bot messages are NOT E2EE.** Bots use `plaintext` column. Bots cannot initiate DMs.
- **Safety numbers:** SHA-256 fingerprint of both parties' identity keys, displayed as numeric code + QR.

### Input Validation
- All Axum handlers use `validator` crate derive macros.
- All input DTOs use `#[serde(deny_unknown_fields)]`.
- SQL injection is prevented by `sqlx::query!` parameterized queries (compile-time verified).
- XSS is mitigated by never rendering user HTML — all user content is plaintext or markdown rendered client-side.

### Headers
- HSTS, CSP, X-Frame-Options, X-Content-Type-Options: nosniff — all via `tower-http`.
- CORS allowlist: `yapperhq.com`, `tauri://localhost`, `capacitor://localhost`.

### Secrets
- Never committed to git. Always in Fly.io secrets or Cloudflare Worker secrets.
- Never logged. `tracing` must never log tokens, passwords, PII, or key material.
- `.env.example` documents all required vars without values.

---

## 5. Project Structure

```
d:\Development\Claude\yapper\
├── marketing/                  ← Phase 0: Astro static site (Cloudflare Pages)
│   ├── src/pages/index.astro
│   ├── src/components/         ← Hero, FeatureGrid, WishlistForm.svelte, etc.
│   ├── worker/wishlist.js      ← Cloudflare Worker: email → D1 → Resend
│   ├── astro.config.mjs
│   └── wrangler.toml
├── backend/                    ← Rust (Axum + Tokio)
│   ├── src/
│   │   ├── main.rs             ← Tokio entrypoint, Router assembly
│   │   ├── db.rs               ← PgPool + LISTEN/NOTIFY
│   │   ├── hub.rs              ← WebSocket hub (Arc<DashMap<...>>)
│   │   ├── error.rs            ← AppError → IntoResponse
│   │   ├── auth/               ← JWT, OAuth, Argon2id, sessions
│   │   ├── users/              ← Profiles, social graph
│   │   ├── servers/            ← CRUD, memberships, invites, explore
│   │   ├── channels/           ← Channel management
│   │   ├── messages/           ← Ciphertext store + delivery
│   │   ├── keys/               ← Signal Protocol key bundles
│   │   ├── media/              ← R2 pre-signed URL generation
│   │   ├── canvas/             ← Live Canvas: music, polls
│   │   ├── emojis/             ← Custom emoji management
│   │   ├── parental/           ← Child accounts, approval flows
│   │   ├── screentime/         ← OS-level screen time ingestion
│   │   ├── bots/               ← Bot accounts + API keys
│   │   ├── discord/            ← Profile import + bot migration
│   │   └── notifications/      ← FCM/APNs push
│   ├── migrations/             ← Timestamp-based SQL (20260301120000_*.sql)
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── fly.toml
├── frontend/                   ← SvelteKit (static adapter)
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/     ← auth/, chat/, canvas/, explore/, emoji/, profile/, settings/, parental/
│   │   │   ├── stores/         ← auth.ts, messages.ts, ws.ts, parental.ts
│   │   │   ├── api/            ← Typed fetch wrapper
│   │   │   ├── signal/         ← libsignal WASM wrapper + keystore
│   │   │   └── plugins/        ← Capacitor/Tauri plugin bridges
│   │   └── routes/
│   │       ├── (auth)/         ← Login, Register, Onboarding
│   │       ├── (app)/          ← Explore, Servers, DMs, Profile, Settings
│   │       └── parent/         ← Parental dashboard
│   ├── src-tauri/              ← Tauri v2 desktop shell (Rust)
│   ├── ios/                    ← Capacitor iOS (ScreenTimePlugin.swift)
│   ├── android/                ← Capacitor Android (ScreenTimePlugin.kt)
│   ├── svelte.config.js
│   ├── vite.config.ts
│   └── package.json
└── docker-compose.yml          ← PostgreSQL 16 only (no Redis)
```

### Module Pattern (Backend)

Every backend feature module follows the same structure:

```
backend/src/feature_name/
├── mod.rs          ← pub mod handlers; pub mod service; (+ router fn)
├── handlers.rs     ← Axum handlers: parse request → call service → Response
├── service.rs      ← Business logic: validation, DB queries, computation
└── types.rs        ← (optional) Request/response DTOs, if not in handlers.rs
```

---

## 6. Database

### Provider
**Neon** (free tier): serverless PostgreSQL, 0.5GB storage, auto-suspend after 5 min inactivity.

Mitigations for Neon cold starts (500ms–2s):
- `sqlx` pool with `min_connections = 1` to keep a warm connection
- Use Neon's PgBouncer pooler endpoint
- If unacceptable, upgrade to Neon Always-On or migrate to Fly Postgres

### Schema Highlights

- **`messages` table:** Stores `ciphertext BYTEA` (E2EE) OR `plaintext TEXT` (bots only). Never both. `CHECK` constraint enforces at least one is non-null.
- **`messages.delivered`:** Boolean flag for offline delivery. On reconnect, hub queries undelivered messages and pushes them.
- **No `media_r2_key_encrypted` column.** Media keys are embedded inside the Signal ciphertext. The server cannot map R2 objects to messages.
- **`friendships` table:** `CHECK(user_id_1 < user_id_2)` prevents duplicate friendships in both directions.
- **Parental tables:** `pending_friend_requests`, `pending_server_joins` — status-based approval workflow.
- **Signal keys:** `identity_keys`, `signed_prekeys`, `one_time_prekeys` — OPKs marked consumed on fetch.

### Migration Rules
- Timestamp-based naming: `20260301120000_description.sql`
- Run via `sqlx migrate run`
- Never edit an existing migration after it has been applied to any environment
- Always create indexes in the same migration as the table (see Required Indexes section in plan)

---

## 7. Phase Roadmap

| Phase | Name | Status | Key Deliverable |
|-------|------|--------|-----------------|
| 0 | Marketing Website | ✅ Complete | Astro site + wishlist on Cloudflare (yapperhq.com live) |
| 1 | Repo Scaffolding | ✅ Complete | Runnable skeleton: backend + frontend + hot reload + CI |
| 2 | Authentication | ✅ Complete | Device-aware register/login, device-bound JWT refresh, OAuth attach-device, pending-trust approval flow |
| 3 | E2EE Core (1:1 DMs) | ✅ Complete | Signal Protocol DMs, WebSocket hub, X3DH + ratchet, PIN backup |
| 4 | Servers & Groups | ✅ Complete | Server/channel CRUD, Sender Keys group E2EE, invite links |
| 5 | Media Messages | ✅ Complete | R2 credentials staged; real-time typing, read receipts, presence |
| 6 | Real-Time Features | ✅ Complete | Typing indicators (5s auto-stop), away detection, presence dots |
| 7 | Live Canvas | ✅ Complete | Music state, polls (live bar animation), clips carousel |
| 8 | Explore Page | ✅ Complete | Search (pg_trgm), trending tags (5-min cache), live servers |
| 9 | User Profiles | ✅ Complete (BE + FE) | Public profiles, follow/unfollow, Hype Moments, BioCard, top communities |
| 10 | Parental Controls | ✅ Complete (BE + FE) | Child accounts (COPPA DOB), approval workflows, SafetyDashboard, 3-step setup wizard (wizard now collects username/email/password for full `CreateChildInput` payload) |
| 11 | Screen Time | FE ✅ — BE Pending | `ScreenTimeDashboard.svelte` built; iOS/Android plugins + BE ingestion API pending |
| 12 | Discord Integration | FE ✅ — BE Pending | `DiscordImport.svelte` + bot migration tool UI built; BE importer + bots/ module pending |
| 13 | Custom Emojis | ✅ Complete (BE + FE) | `EmojiPicker`, `EmojiUploader`, `CustomEmojiManager` built; BE emojis/ complete; emoji rendering in MessageList (XSS-safe `:shortcode:` → `<img>`), emoji picker in MessageInput wired end-to-end |
| 14 | User Settings | ✅ Complete (Appearance + Notifications) — Partial | Appearance + Notifications: DB tables created, `GET/PATCH /api/v1/users/me/appearance|notifications` implemented, FE loads/saves live. Still pending: GDPR data export, profile avatar/banner upload, soft-delete |
| 15 | Tauri Polish | FE Partial | `TitleBar.svelte` + `KeyboardShortcutsModal.svelte` done; system tray, auto-updater, deep links pending |
| 16 | Security Audit | Pending | Pre-launch hardening, GDPR/COPPA compliance verification, `SECURITY_AUDIT.md` |
| 17 | Premium Placeholder | FE ✅ — BE Pending | `Premium.svelte`, `GoproLock.svelte`, settings GoPro promo card built; BE `is_premium` flag pending |
| 18 | Launch Prep | Pending | Fly.io production deploy, Sentry, E2E test suite, app store submissions |
| 19 | Support Tickets | ✅ Complete | `POST/GET /api/v1/support/tickets`, HubSpot CRM integration, `Support.svelte` settings page |
| 20 | Build Pipeline | ✅ Complete | GHA Docker layer cache, `flyctl deploy --image`, Cargo profile tuning (~90s deploys) |

### Global UI Infrastructure (Complete — 2026-03-03)

The following cross-cutting UI components were built and wired into `(app)/+layout.svelte`:

| Component | Purpose |
|-----------|---------|
| `Toast.svelte` + `stores/toast.ts` | Bottom-right notifications, 4 types, auto-dismiss |
| `Skeleton.svelte` | Shimmer loader for async content |
| `ContextMenu.svelte` | Cursor-positioned context menu, Escape to close |
| `AppLoadingScreen.svelte` | Full-screen loading state shown while auth restore, device approval, vault unlock, and Signal bootstrap complete |

### Current Login Flow (Updated 2026-03-09)

- Frontend login and register now use `POST /api/v2/auth/login` and `POST /api/v2/auth/register`.
- Every auth entrypoint sends device bootstrap metadata from the current install: stable `installation_id`, detected `platform`, and a local device `label`.
- Auth responses now return `access_token`, `csrf_token`, `user`, and `device` metadata. The frontend stores the active device and reuses the same local Signal vault on the same machine/profile.
- Session restore now uses `POST /api/v2/auth/refresh`. The refresh cookie is scoped to `/api/v2/auth/refresh`, and the backend binds the refreshed session to the active `device_id`.
- OAuth callback no longer finishes login on its own. After the provider callback lands, the frontend calls `POST /api/v2/auth/attach-device` so the signed-in account is attached to the current installation.
- New installs do not become fully trusted automatically. A new device can enter `pending_trust`, which blocks normal chat/message access until an existing trusted device approves it or the user restores an encrypted backup.
- Trusted devices load `/api/v2/devices`, show the device approval inbox, and can approve pending devices with `POST /api/v2/devices/:id/approve`.
- Normal logout is auth-only. It clears session state but does not wipe the local E2EE vault. Device revocation / "forget this device" is the destructive path that revokes the device and removes its local key material.

---

## 8. Development Environment Setup

### Prerequisites
1. **Rust 1.93+** — `rustup update stable`
2. **Node.js 20+** — for SvelteKit + Astro
3. **Docker** — for local PostgreSQL
4. **cargo-watch** — `cargo install cargo-watch`
5. **sqlx-cli** — `cargo install sqlx-cli --no-default-features --features postgres`
6. **Tauri CLI v2** — `npm install -g @tauri-apps/cli@2`
7. **Capacitor CLI** — `npm install -g @capacitor/cli@latest`

### Running Locally
```bash
# 1. Start PostgreSQL
docker compose up -d

# 2. Run migrations
cd backend && sqlx migrate run

# 3. Start backend (hot reload)
cargo watch -x run
# → API + WebSocket at localhost:8080

# 4. Start frontend (separate terminal)
cd frontend && npm run dev
# → SvelteKit at localhost:5173

# 5. Desktop app (separate terminal)
cd frontend && npm run tauri dev
# → Tauri window with SvelteKit inside
```

### Required Environment Variables
See `.env.example` for the full list. Key ones:
```
DATABASE_URL=postgres://yapper:yapper@localhost:5432/yapper
JWT_PRIVATE_KEY=<RS256 private key PEM>
JWT_PUBLIC_KEY=<RS256 public key PEM>
R2_ACCOUNT_ID=<cloudflare account id>
R2_ACCESS_KEY_ID=<r2 api token>
R2_SECRET_ACCESS_KEY=<r2 api secret>
R2_BUCKET_NAME=yapper-media
RESEND_API_KEY=<resend api key>
FCM_SERVICE_ACCOUNT_JSON=<firebase service account>
DISCORD_CLIENT_ID=<discord oauth app id>
DISCORD_CLIENT_SECRET=<discord oauth app secret>
```

---

## 9. Testing Requirements

Testing is embedded from Phase 1, not an afterthought.

| Layer | Tool | Command | Gate |
|-------|------|---------|------|
| Backend unit | `#[cfg(test)]` + `cargo test` | `cargo test` | CI blocks merge on failure |
| Backend integration | `sqlx::test` (disposable test DB) | `cargo test` | CI blocks merge on failure |
| Backend lint | Clippy | `cargo clippy -- -D warnings` | CI blocks merge on warnings |
| Backend format | rustfmt | `cargo fmt --check` | CI blocks merge on diff |
| Backend security | cargo-audit | `cargo audit` | CI blocks merge on known CVEs |
| Frontend unit | Vitest + @testing-library/svelte | `npm run test` | CI blocks merge on failure |
| Frontend lint | ESLint + svelte-check | `npm run lint && npm run check` | CI blocks merge on failure |
| Frontend security | npm audit | `npm audit` | CI warns (review manually) |
| E2E | Playwright | `npx playwright test` | CI blocks merge on failure |

---

## 10. Deployment

### Backend (Fly.io)
```bash
cd backend
fly deploy                    # Deploys Dockerfile → Fly.io
fly secrets set KEY=VALUE     # Set env vars (never in code)
fly logs                      # Tail logs
fly status                    # Check VM health
```

### Frontend (Cloudflare Pages)
- Auto-deploys on `git push` to `main`
- Preview deployments on every PR (`*.pages.dev`)
- Custom domain: `app.yapperhq.com`

### Marketing Site (Cloudflare Pages)
- Auto-deploys on `git push` to `main` (separate Pages project)
- Custom domain: `yapperhq.com`

### Wishlist Worker (Cloudflare Workers)
```bash
cd marketing
npx wrangler deploy           # Deploy worker
wrangler secret put RESEND_API_KEY  # Set secret
```

---

## 11. Critical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Apple FamilyControls entitlement (4+ weeks) | iOS Screen Time blocked | Apply immediately; soft-launch iOS without it |
| Neon 0.5GB storage limit | DB full at ~1000 users | Monitor `pg_database_size()`; upgrade at 400MB |
| In-memory hub lost on VM restart | Active connections drop | Clients auto-reconnect; undelivered messages replayed from PostgreSQL |
| Single Fly VM | Single point of failure | Acceptable for MVP; scale to 2 VMs later (requires Redis for hub) |
| libsignal WASM init (~200ms) | First message delayed | Initialize on app load (background); cache module |
| Neon cold starts (500ms–2s) | First request slow | Keep `min_connections = 1`; use PgBouncer endpoint |

---

## 12. Cost Summary

**Monthly cost: $0** (all free tiers)

**Unavoidable one-time costs (only at app store submission):**
- Domain: ~$10/year
- Apple Developer: $99/year
- Google Play: $25 one-time

**When costs begin:**
- Neon storage > 0.5GB → $19/month (Launch plan)
- R2 storage > 10GB → $0.015/GB
- Fly.io extra VMs → ~$5.70/month each
- Resend > 3K emails/month → $20/month

---

## 13. Glossary

| Term | Definition |
|------|-----------|
| **Yap** | Audio voice message (recorded + E2EE + sent via R2) |
| **Clip** | Video message (recorded + E2EE + sent via R2) |
| **Live Canvas** | Right-side panel in server view: music state, polls, clips carousel |
| **Hype Moments** | Pinned messages on a user's profile (like a highlight reel) |
| **GoPro** | Premium tier name (placeholder, no payment integration yet) |
| **Safety Gates** | Parental control toggles (auto-hold DMs, community join approval, etc.) |
| **Sender Keys** | Signal Protocol mechanism for efficient group encryption (encrypt once, all members decrypt) |
| **X3DH** | Extended Triple Diffie-Hellman — Signal's key agreement protocol for establishing DM sessions |
| **OPK** | One-Time Prekey — consumed on first message to a user; client uploads batches of 100 |
| **DashMap** | Lock-free concurrent hash map (Rust crate) — used for the in-memory WebSocket hub |
| **Hub** | The in-memory WebSocket connection registry (`Arc<DashMap<UserId, HashMap<DeviceId, Sender>>>`) |

---

## 14. Handover Checklist

For any developer picking up this project:

- [ ] Read this document fully
- [ ] Read the full implementation plan at `C:\Users\rajma\.claude\plans\quizzical-yawning-starfish.md`
- [ ] Review the sprint plan at `dev docs/SPRINT_PLAN.md` for current task status
- [x] Complete the Pre-Phase setup (accounts, tools, entitlements)
- [x] Run `docker compose up -d` and verify PostgreSQL is healthy
- [x] Understand the E2EE constraint: **server never sees plaintext**
- [x] Understand the parental controls constraint: **metadata only**
- [x] Review the database schema, especially the `messages` table and Signal key tables
- [x] Review the security standards in Section 4 — these are non-negotiable
- [ ] Check the current phase status and pick up where it left off

### Where to Pick Up Next (as of 2026-03-16, rev 3)

**S0–S12 complete. Support tickets live. Build pipeline optimised.** Priority order:

1. **E2E Testing** — Write Playwright tests (`e2e-nightly.yml` is configured, test accounts needed: `E2E_USER_EMAIL/PASSWORD` as GitHub Secrets)
2. **macOS DMG build** — Run on Mac: `cargo tauri build --target universal-apple-darwin`
3. **iOS build** — Run on Mac: `cd frontend/ios && pod install`, then Xcode archive + App Store Connect upload
4. **Google Play submission** — Build AAB, create Play Console listing ($25 one-time)
5. **Marketing site update** — Update hero copy, add download links for Windows installer
6. **Wishlist email blast** — Send launch announcement to all wishlist subscribers via Resend
7. **Generate Tauri signing keys** — `cargo tauri signer generate`, set `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PASSWORD` as GitHub Secrets
8. **Apple OAuth credentials** — Create Apple Sign-In service ID, configure redirect URIs

**Completed since last handover (2026-03-03 rev 2 → rev 3):**
- Support tickets: `POST/GET /api/v1/support/tickets`, migration 000026, HubSpot CRM Tickets API (`HUBSPOT_ACCESS_TOKEN` on Fly.io), `Support.svelte` with type selector/priority chips/ticket history in settings
- Build pipeline: GHA Docker layer caching (`type=gha,mode=max`), `flyctl deploy --image <sha>`, Cargo profile tuning (`codegen-units=16, lto=false`). Deploy time: ~700s → ~90s steady-state
- All 17 Dependabot PRs merged — npm/Cargo/GH Actions fully up to date
- Project management: Linear integration guide at `dev docs/LINEAR_INTEGRATION.md`
- Docs: `dev docs/BUILD_SPEED_OPTIMISATION.md`, `dev docs/LINEAR_INTEGRATION.md`
