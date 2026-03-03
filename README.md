# Yapper

**A New Way to Yap.** End-to-end encrypted real-time chat with servers, channels, audio/video, and COPPA-compliant parental controls — deployable for $0/month.

> Live: [yapperhq.com](https://yapperhq.com) · App: [app.yapperhq.com](https://app.yapperhq.com) · API: [api.yapperhq.com](https://api.yapperhq.com)

---

## What is Yapper?

Yapper is a Discord-like communication platform built from the ground up with **end-to-end encryption** as a non-negotiable. Every direct message and channel message is encrypted on-device using the Signal Protocol (X3DH + Double Ratchet). The server sees only ciphertext — never plaintext.

It ships as:
- **Web PWA** — runs in any browser
- **Desktop** — Tauri v2 native app (Windows, macOS, Linux)
- **Mobile** — Capacitor (iOS + Android)

---

## Features

| Feature | Status |
|---------|--------|
| E2EE direct messages (X3DH + AES-256-GCM) | ✅ |
| E2EE servers & channels (Sender Keys) | ✅ |
| Real-time WebSocket hub (typing, read receipts, presence) | ✅ |
| OAuth — Discord, Google | ✅ |
| Audio Yaps (voice messages) | 🔜 |
| Video Clips | 🔜 |
| Live Canvas (music widget, polls, clips) | ✅ |
| Explore page with discovery & trending tags | ✅ |
| User profiles, follow graph, Hype Moments | ✅ BE |
| COPPA parental controls (friend/server approval flow) | ✅ BE |
| Discord profile import + bot migration | 🔜 |
| Screen time reporting (iOS FamilyControls, Android UsageStats) | 🔜 |
| Push notifications (FCM) | ✅ |
| Custom server emoji | ✅ |

---

## Tech Stack

### Backend
- **Runtime:** Rust + Tokio (async)
- **Web framework:** Axum
- **Database:** PostgreSQL (Neon serverless) via sqlx
- **Real-time:** In-memory WebSocket hub (`DashMap` + `mpsc` channels)
- **Auth:** JWT RS256 + Argon2id password hashing
- **E-mail:** Resend
- **Media:** Cloudflare R2 (client-side encrypted before upload)
- **Push:** Firebase Cloud Messaging (FCM)
- **Error monitoring:** Sentry

### Frontend
- **Framework:** SvelteKit (static adapter → Cloudflare Pages)
- **Crypto:** `@noble/curves` + `@noble/hashes` + Web Crypto API
- **Desktop:** Tauri v2
- **Mobile:** Capacitor v8

### Infrastructure (all free tier)
| Service | What it does |
|---------|-------------|
| Fly.io | Backend (Rust binary, always-on) |
| Neon | PostgreSQL (0.5 GB free) |
| Cloudflare Pages | Frontend + Marketing site |
| Cloudflare R2 | Encrypted media (10 GB free) |
| Cloudflare Workers + D1 + KV | Wishlist API |
| Resend | Transactional email (3K/month) |
| Firebase | FCM push notifications |

---

## Repository Layout

```
yapper/
├── backend/             # Rust/Axum API server
│   ├── src/
│   │   ├── main.rs      # Entry point, router, middleware
│   │   ├── hub.rs       # WebSocket hub (all real-time fan-out)
│   │   ├── auth/        # Register, login, OAuth, JWT, CSRF
│   │   ├── users/       # Profiles, follow graph, hype moments
│   │   ├── servers/     # Server + channel CRUD, invite links
│   │   ├── channels/    # Channel messages (encrypted)
│   │   ├── messages/    # Direct message conversations
│   │   ├── keys/        # Signal Protocol key server
│   │   ├── canvas/      # Live Canvas (music, polls, clips)
│   │   ├── explore/     # Discovery, search, trending
│   │   ├── parental/    # COPPA parental controls
│   │   ├── media/       # R2 presigned upload/download
│   │   ├── emojis/      # Custom server emoji
│   │   ├── notifications/ # Push notification dispatch
│   │   ├── screentime/  # Screen time reporting
│   │   ├── bots/        # Bot application management
│   │   └── discord/     # Discord import
│   └── migrations/      # sqlx SQL migrations (12 files)
│
├── frontend/            # SvelteKit app (PWA + Tauri + Capacitor)
│   ├── src/
│   │   ├── lib/
│   │   │   ├── signal/  # X3DH, ratchet, keystore (IndexedDB)
│   │   │   ├── stores/  # Svelte writable stores (ws, servers, canvas…)
│   │   │   ├── api/     # Typed API client
│   │   │   └── components/
│   │   └── routes/
│   │       ├── (auth)/  # Login, register, OAuth callback
│   │       └── (app)/   # Main app shell (servers, DMs, explore…)
│   ├── ios/             # Capacitor iOS project
│   └── android/         # Capacitor Android project
│
├── marketing/           # Astro 4 marketing site → Cloudflare Pages
├── scripts/             # DB seed, key generation helpers
├── dev docs/            # Sprint plan + full implementation plan
├── docker-compose.yml   # Local PostgreSQL
├── Makefile             # Common dev tasks
└── .env.example         # All required environment variables
```

---

## Quick Start

### Prerequisites

- Rust 1.78+ (`rustup`)
- Node.js 20+ + npm
- Docker (for local PostgreSQL)
- `sqlx-cli`: `cargo install sqlx-cli --no-default-features --features postgres`

### 1. Clone + configure

```bash
git clone https://github.com/NikheelR97/Yapper.git
cd Yapper
cp .env.example backend/.env
```

Edit `backend/.env` — at minimum set `DATABASE_URL`.

### 2. Generate JWT keys

```bash
cd backend/secrets
openssl genrsa -out jwt_private.pem 2048
openssl rsa -in jwt_private.pem -pubout -out jwt_public.pem
```

### 3. Start PostgreSQL + run migrations

```bash
make db-up        # starts Docker postgres on :5432
make migrate      # runs all sqlx migrations
```

### 4. Start the backend

```bash
make dev-backend  # cargo watch -x run  →  localhost:8080
```

### 5. Start the frontend

```bash
cd frontend && npm install
make dev-frontend # →  localhost:5173
```

### 6. (Optional) Desktop app

```bash
make dev-tauri    # Tauri window + hot-reload
```

---

## Common Make Targets

| Command | What it does |
|---------|-------------|
| `make dev` | Docker postgres + backend hot-reload |
| `make dev-frontend` | SvelteKit dev server |
| `make dev-tauri` | Tauri desktop dev |
| `make migrate` | Apply pending DB migrations |
| `make sqlx-prepare` | Regenerate sqlx offline query cache |
| `make test` | Backend + frontend tests |
| `make lint` | clippy + eslint |
| `make fmt` | cargo fmt + prettier |
| `make deploy` | Deploy backend (Fly.io) + frontend (CF Pages) |

---

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | Full stack diagram, module responsibilities, data flow |
| [E2EE & Security](docs/e2ee.md) | Signal Protocol implementation, key lifecycle, threat model |
| [API Reference](docs/api.md) | All REST endpoints + WebSocket message types |
| [Development Guide](docs/development.md) | Local setup, migrations, testing, code conventions |
| [Deployment](docs/deployment.md) | Fly.io, Cloudflare Pages, Neon, environment variables |

---

## Environment Variables

See [`.env.example`](.env.example) for the full list. Required for local dev:

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string |
| `JWT_PRIVATE_KEY_PATH` | Path to RSA-2048 private key PEM |
| `JWT_PUBLIC_KEY_PATH` | Path to RSA-2048 public key PEM |
| `RESEND_API_KEY` | Transactional email |
| `DISCORD_CLIENT_ID` / `_SECRET` | Discord OAuth |
| `GOOGLE_CLIENT_ID` / `_SECRET` | Google OAuth |
| `R2_*` | Cloudflare R2 media bucket |

---

## Security Model

- **E2EE is inviolable** — the server cannot read message content.
- **Parental controls operate on metadata only** — parents see who their child is talking to and what servers they're joining, never message content.
- CSRF double-submit cookie on all state-mutating endpoints.
- Argon2id (memory-hard) for password hashing.
- JWT RS256 with 15-minute access tokens + 30-day refresh tokens.
- Rate limiting: 100 req/min per IP (API) + 5 msg/sec per user (WebSocket).
- All secrets stored in Fly.io secret store (never committed).

See [docs/e2ee.md](docs/e2ee.md) for the full threat model.

---

## Project Status

Actively in development. Tracking progress in [`dev docs/SPRINT_PLAN.md`](dev%20docs/SPRINT_PLAN.md).

| Sprint | Theme | Status |
|--------|-------|--------|
| S0 — Setup + Marketing | Marketing site, wishlist | ✅ ~98% |
| S1 — Scaffolding | Repo, CI, DB schema | ✅ |
| S2 — Auth | Register, login, OAuth, JWT | ✅ |
| S3 — E2EE Core | Signal Protocol, DMs | ✅ |
| S4 — Servers & Channels | Group E2EE, invite links | ✅ |
| S5 — Real-Time | Typing, read receipts, presence | ✅ |
| S6 — Live Canvas + Explore | Canvas widgets, discovery | ✅ |
| S7 — Profiles + Parental | Social graph, COPPA controls | 🔄 BE done |
| S8 — Screen Time + Discord | OS screen time, Discord import | 🔜 |
| S9–S11 — Polish + Launch | Emojis, settings, premium, launch | 🔜 |

---

## License

Proprietary — all rights reserved. Not open source.
