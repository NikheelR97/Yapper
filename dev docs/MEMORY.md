# Yapper Project Memory

## Project
**App name:** Yapper
**Type:** Greenfield SaaS real-time E2EE chat platform
**Root dir:** `d:\Development\Claude\yapper\`
**Plan file:** `C:\Users\rajma\.claude\plans\quizzical-yawning-starfish.md`

## Key Architecture Decisions
- **Backend:** Rust + Axum + Tokio — single binary `src/main.rs` (API + WebSocket hub combined for MVP)
  - Rust already in stack (Tauri); libsignal is native Rust; no GC pauses; ~10MB idle RAM vs Go's ~50MB (critical for Fly.io 256MB VM)
- **Frontend:** SvelteKit (static adapter) → Web PWA, Tauri v2 desktop, Capacitor mobile
- **E2EE:** `@noble/curves` + `@noble/hashes` + Web Crypto API — X3DH + symmetric ratchet + AES-256-GCM. (`@signalapp/libsignal-client` is NAPI-only, not usable in WebView)
- **Database:** Neon (free PostgreSQL) — no Redis for MVP
- **Real-time:** In-memory Rust hub (`Arc<RwLock<HashMap>>` + Tokio `mpsc` channels) + PostgreSQL LISTEN/NOTIFY (no Redis, no SpaceTimeDB)
- **Media:** Cloudflare R2 (10GB free) — client-side AES-256-GCM encrypt before upload
- **Hosting:** Fly.io (3 free always-on VMs) — no Render
- **Marketing site:** Astro 4 → Cloudflare Pages; wishlist via Cloudflare Worker + D1 + KV + Resend
- **SpaceTimeDB:** Evaluated, deferred to post-MVP (1GB limit + compliance needs PostgreSQL anyway)

## Free Tier Stack (total: $0/month)
- Fly.io: backend (always-on, no sleep)
- Neon: PostgreSQL (0.5GB free)
- Cloudflare Pages: frontend + marketing site
- Cloudflare R2: media (10GB free)
- Cloudflare Workers + D1 + KV: wishlist API
- Resend: transactional email (3K/month free)
- FCM: push notifications (free)
- Sentry: error monitoring (5K errors/month free)

## Unavoidable Costs
- Domain: ~$10/year
- Apple Developer: $99/year (only at App Store submission)
- Google Play: $25 one-time (only at Play Store submission)

## Parental Controls (Critical Design)
- E2EE is inviolable — parents see METADATA ONLY
- Friend requests to child → `pending_friend_requests` table → parent approves before E2EE session established
- Server joins by child → `pending_server_joins` → parent approves before membership granted
- Content moderation: REMOVED from MVP

## User Preferences
- Maximize free tiers — only pay if absolutely necessary
- All 3 media types in MVP: Audio Yap, Video Clip, Live Canvas
- Screen time: OS-level (iOS DeviceActivity + Android UsageStatsManager)
- Discord profile import + bot migration tool included

## Firebase
- **Project:** `yapper-41f63`
- **Service account:** `backend/secrets/firebase-service-account.json` (gitignored)
- **Fly.io secret:** `FCM_SERVICE_ACCOUNT_JSON` staged
- **VAPID public key:** `BOrz15S2L_kg0_R5Tam_gbtO_Zf3LzTVFwzQGbCsDFLZgi6fG5DUk3KjIMLs4N3oGeaGkJcbxfeBlHGzDF5k8Cc`
- **Frontend env:** `VITE_FIREBASE_VAPID_KEY` in `frontend/.env`

## Discord OAuth
- **Client ID:** `1477680877428539538`
- **Client Secret:** in `backend/.env` + staged as Fly.io secret
- **Redirect URIs:** `http://localhost:8080/auth/oauth/discord/callback` + `https://api.yapperhq.com/auth/oauth/discord/callback`

## Google OAuth
- **Project:** `yapper-41f63` (same as Firebase)
- **Client ID:** `1038109147262-g4l896nrvlvqlou0sseh59vh9cnpb15v.apps.googleusercontent.com`
- **Client Secret:** in `backend/.env` + staged as Fly.io secret
- **Redirect URIs:** `http://localhost:8080/auth/oauth/google/callback` + `https://api.yapperhq.com/auth/oauth/google/callback`

## Phase Progress
- **S0** (Setup + Marketing): ~98% done — site live, wishlist emails working, Lighthouse done (Perf 91, A11y 100, BP 81, SEO 100). Remaining: Apple OAuth creds, Apple FamilyControls entitlement
- **S1** (Scaffolding): ✅ Complete — `cargo sqlx prepare` run, `.sqlx/` cache committed
- **S2** (Auth): ✅ Complete — full backend auth (register/login/refresh/logout/email-verify/password-reset), Discord+Google OAuth, JWT RS256 keys in `backend/secrets/`, frontend auth pages + OAuth callback, unit tests in service.rs+middleware.rs. Deferred: Apple OAuth, Playwright E2E test
- **S3** (Signal Protocol & E2EE Core): ✅ Complete — backend key server (5+2 endpoints), conversations/messages API, hub SendDm + offline delivery + WS per-user rate limiting (5msg/sec, burst 20), CSRF double-submit middleware (`backend/src/csrf.rs`), PIN key backup (`migration 000010` + `GET/PUT /api/v1/keys/backup` + `frontend/src/lib/signal/backup.ts`), frontend X3DH + symmetric ratchet (AES-256-GCM), IndexedDB keystore, ws.ts + conversations.ts stores, DM page + MessageList + MessageInput. Deferred: backend tests, Apple OAuth, Playwright E2E.
- **Capacitor**: ✅ iOS + Android platforms added (`frontend/ios/` + `frontend/android/`); @capacitor/core@8, @capacitor/android@8, @capacitor/ios@8, @capacitor/push-notifications@8. CocoaPods pod install skipped on Windows — run on Mac before iOS build.
- **S4** (Servers, Channels & Group E2EE): ✅ Complete — Sender Keys (HMAC-SHA256 chain + Ed25519 signing + ECIES key dist), `stores/servers.ts`, `ServerSidebar.svelte`, channel chat page, server creation modal, invite links, join-by-invite, `registerChannelHandler` in app layout. Wire format: `base64(sig_64 || aes_ct)`. `prepareChannel()` is idempotent — joins if no key, fetches pending dists if key exists. Deferred: tests, icon upload in server create modal, child account server join interception (→S7)
- **S5/W12** (Real-Time Features): ✅ Complete — typing indicators (`Hub.typing_timers: DashMap<(channel_id, user_id), JoinHandle>`, 5s auto-stop, fan-out `Typing`/`TypingStop`), read receipts (upsert + fan-out `ReadReceipt`), `GET /api/v1/users/:id/presence`, `TypingIndicator.svelte` (bounce-dot animation), `MessageInput` channelId prop + 2s-throttled `sendTypingStart`. Away detection: 5-min inactivity timer (`away_timers: DashMap`, `away_users: DashMap`), 3-state presence (online/away/offline). Deferred: ReadReceipt.svelte UI, presence dots on avatars, IntersectionObserver read marking.
- **S5/W11** (E2EE Media — Audio Yaps/Video Clips): R2 credentials staged (`R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` in backend/.env). R2 CORS + lifecycle configured via Cloudflare dashboard. Media upload implementation deferred.
- **S6** (Live Canvas + Explore): ✅ Complete — Canvas BE (`backend/src/canvas/mod.rs`): PATCH music, GET canvas+clips, POST polls/vote, WS `CanvasUpdate` fan-out. Explore BE (`backend/src/explore/mod.rs`): communities, live-servers, trending-tags (5-min in-memory cache), pg_trgm search. Migration 000012: `tags TEXT[]` on servers + GIN user search index. Canvas FE: `canvas.ts` store, `LiveCanvas.svelte` (right panel + toggle), `MusicWidget.svelte` (spinning art + EQ bars), `PollWidget.svelte` (fill-bar animation), `ClipsCarousel.svelte`. Explore FE: `explore.ts` store, full `explore/+page.svelte` (search bar 350ms debounce, grid/list toggle, tag filter), `TrendingTags.svelte`, `LiveServerCard.svelte`, `CommunityCard.svelte`. WS: `canvas_update` case added to ws.ts, `registerCanvasHandler()` in app layout. Deferred: BE tests, `top-yappers` endpoint (needs S7 followers table).
- **S7** (Profiles + Parental Controls): ✅ Complete (BE + FE) — BE: users/mod.rs (follow/unfollow/hype-moments/profile), parental/mod.rs (COPPA child create, approval workflows, audit trail), parental interception in servers/service.rs. FE: `profile/[username]/+page.svelte`, `ProfileHeader`, `BioCard`, `HypeMoments`, `TopCommunitiesCard`, `MutualConnectionsCard` (`stores/profile.ts`); parent layout (`routes/parent/`), `SafetyDashboard`, `PendingAlerts`, `SafetyFeed`, `ActivitySnapshot`, 3-step child setup wizard (`stores/parental.ts`). Deferred: BE tests, Playwright E2E.
- **S8** (Screen Time + Discord): FE ✅ — `ScreenTimeDashboard.svelte` (period tabs, SVG chart, limit slider), `DiscordImport.svelte` (connected accounts: Discord/Google/Apple), `DeveloperTools.svelte` (bot migration 2-step). BE pending: `src/screentime/`, `src/discord/importer.rs`, `src/bots/`.
- **S9** (Emojis + Settings): FE ✅ — `EmojiPicker.svelte` (recent+server+Unicode tabs, 8-col grid, search), `EmojiUploader.svelte` (drag-drop, auto-slugify, FormData upload), `CustomEmojiManager.svelte` (list, delete, GoPro limit banner). Settings: `(app)/settings/+page.svelte` 3-col layout + `ProfileForm`, `PrivacySafety`, `Appearance`, `VoiceVideo`, `Notifications`, `Premium`, `DiscordImport`, `DeveloperTools` in `lib/components/settings/`. BE pending: emojis/ (WebP conv, R2, WS events), settings save endpoints, GDPR export/delete.
- **S10/S11** (Desktop + Global): FE partial — `TitleBar.svelte` (Tauri-only, drag region, W11 minimize/maximize/close), `KeyboardShortcutsModal.svelte` (Ctrl+/ trigger, 4 sections), `GoproLock.svelte` (blur overlay, centered card), `AppLoadingScreen.svelte` (sphere pulse, indeterminate progress bar). Global: `Toast.svelte`+`stores/toast.ts`, `Skeleton.svelte`, `ContextMenu.svelte` all wired into `(app)/+layout.svelte`. Deferred: system tray, auto-updater, deep links, Sentry, E2E tests.

## S3 Key Implementation Notes
- **@noble/curves v2**: `randomPrivateKey` → `randomSecretKey`; all imports need `.js` extension (`@noble/curves/ed25519.js`, `@noble/hashes/hkdf.js`)
- **Web Crypto + noble types**: `hmac()` returns `Uint8Array<ArrayBufferLike>` — call `.slice()` before passing to `crypto.subtle.importKey/encrypt/decrypt` (needs `Uint8Array<ArrayBuffer>`)
- **sqlx new queries**: Use `sqlx::query()` (non-macro) + `Row::try_get()` for new S3 queries; avoids needing `cargo sqlx prepare` for every change
- **OPK consumption**: `FOR UPDATE SKIP LOCKED` in PostgreSQL for atomic one-time prekey consumption
- **Conversations route**: mounted at `/conversations` (not `/messages`) in main.rs
- **Migration**: `20260301000009_add_signing_key.sql` adds `signing_key BYTEA` to `identity_keys`

## S2 Key Implementation Notes
- **OAuth routes:** at `/auth/oauth/{provider}` (top-level, NOT `/api/v1/`), to match registered redirect URIs
- **OAuth provider storage:** `discord_id` column stores `"provider:id"` format (e.g. `discord:123456`, `google:987654`)
- **sqlx offline cache:** `.sqlx/` directory populated via `cargo sqlx prepare` — must use non-pooler Neon endpoint for `cargo sqlx prepare` (pooler = PgBouncer = no prepared statements)
- **Neon direct endpoint:** `ep-broad-block-a9johezc.gwc.azure.neon.tech` (no `-pooler`) for migrations + sqlx prepare
- **JWT keys:** RSA-2048 at `backend/secrets/jwt_private.pem` + `jwt_public.pem` (gitignored; load via `JWT_PRIVATE_KEY_PATH` in dev, `JWT_PRIVATE_KEY` env var in prod)
- **validator crate:** v0.18 `AsRegex` trait doesn't work with `once_cell::Lazy<Regex>` — use manual `if !REGEX.is_match(...)` check instead

## Fly.io
- **App:** `yapper-api` — region `jnb` (Johannesburg)
- **Auth:** `rajmannikheel@gmail.com`
- **CLI:** `C:\Users\rajma\.fly\bin\flyctl.exe`
- **DATABASE_URL secret:** staged (Neon connection string)

## Neon
- **Project:** `neondb` — pooler endpoint `ep-broad-block-a9johezc-pooler.gwc.azure.neon.tech`
- **Local .env:** `backend/.env` has DATABASE_URL set

## Cloudflare Resources (all live)
- **Account ID:** `3cb90d502ed2d67a8dadec9dac386425`
- **Zone ID (yapperhq.com):** `66c72c02b2e495f96dc3bab171051790`
- **D1:** `yapper-wishlist` — ID `a13f92ef-9c9c-4006-a503-1188c843bc00` — schema migrated
- **KV:** `yapper-counters` — ID `02cc3d0ec89449b0bdc82efe93e4754b`
- **Worker:** `yapper-wishlist-api` — live at `yapperhq.com/api/*`
- **R2:** `yapper-media` — created
- **Pages:** `yapper-marketing` (yapperhq.com) + `yapper-app` (app.yapperhq.com) — both connected to GitHub NikheelR97/Yapper
- **DNS:** apex → yapper-marketing.pages.dev, www → yapper-marketing.pages.dev, app → yapper-app.pages.dev
