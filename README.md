# Yapper

**A New Way to Yap.** End-to-end encrypted real-time chat with servers, channels, audio/video, and COPPA-compliant parental controls — deployable for $0/month.

> Live: [yapperhq.com](https://yapperhq.com) · App: [app.yapperhq.com](https://app.yapperhq.com) · API: [api.yapperhq.com](https://api.yapperhq.com/health)

[![CI](https://github.com/NikheelR97/Yapper/actions/workflows/ci.yml/badge.svg)](https://github.com/NikheelR97/Yapper/actions/workflows/ci.yml)
[![E2E Nightly](https://github.com/NikheelR97/Yapper/actions/workflows/e2e-nightly.yml/badge.svg)](https://github.com/NikheelR97/Yapper/actions/workflows/e2e-nightly.yml)

---

## What is Yapper?

Yapper is a Discord-like communication platform built with **end-to-end encryption** as a non-negotiable. Every direct message and channel message is encrypted on-device using the Signal Protocol (X3DH + Double Ratchet for DMs, Sender Keys for channels). The server stores only ciphertext — never plaintext.

It ships as:
- **Web PWA** — runs in any browser
- **Desktop** — Tauri v2 native app (Windows, macOS, Linux)
- **Mobile** — Capacitor v8 (iOS + Android)

---

## Features

| Feature | Status |
|---------|--------|
| E2EE direct messages (X3DH + Double Ratchet + AES-256-GCM) | ✅ |
| E2EE group channels (Sender Keys) | ✅ |
| Multi-device E2EE (per-device Signal keys + trust workflow) | ✅ |
| Real-time WebSocket hub (typing, read receipts, presence) | ✅ |
| OAuth — Discord, Google | ✅ |
| Live Canvas (music widget, polls, video clips) | ✅ |
| Explore page (server discovery, trending tags, search) | ✅ |
| User profiles, follow graph, Hype Moments | ✅ |
| COPPA parental controls (friend/server approval flow) | ✅ |
| Screen time reporting | ✅ |
| Discord profile import + bot migration | ✅ |
| Custom server emoji (WebP conversion) | ✅ |
| Push notifications (FCM) | ✅ |
| Premium subscription + promo codes | ✅ |
| Support tickets (linked to HubSpot CRM) | ✅ |
| GDPR data export + account deletion | ✅ |
| Audio Yaps (voice messages) | 🔜 |
| Video Clips upload | 🔜 |
| Apple OAuth | 🔜 |

---

## Tech stack

### Backend
| | |
|-|-|
| Language | Rust 1.85 + Tokio async runtime |
| Web framework | Axum 0.7 |
| Database | PostgreSQL 16 (Neon serverless) via sqlx |
| Real-time | In-memory WebSocket hub (`DashMap` + `mpsc` channels) |
| Auth | JWT RS256 + Argon2id + CSRF double-submit |
| Email | Resend |
| Media | Cloudflare R2 (client-side AES-256-GCM encrypted before upload) |
| Push | Firebase Cloud Messaging (FCM) |
| Error monitoring | Sentry |
| CRM | HubSpot (support tickets) |

### Frontend
| | |
|-|-|
| Framework | SvelteKit (static adapter) |
| Crypto | `@noble/curves` + `@noble/hashes` + Web Crypto API |
| Desktop | Tauri v2 |
| Mobile | Capacitor v8 |
| E2E tests | Playwright (sharded, nightly) |

### Infrastructure (all free tier)
| Service | Role |
|---------|------|
| Fly.io | Backend API (2 always-on machines, Johannesburg) |
| Neon | PostgreSQL 0.5 GB |
| Cloudflare Pages | Frontend + marketing hosting |
| Cloudflare R2 | Encrypted media storage (10 GB) |
| Cloudflare Workers + D1 + KV | Wishlist API |
| Resend | Transactional email (3 K/month free) |
| Firebase | FCM push notifications |
| Sentry | Error monitoring (5 K errors/month free) |

---

## Repository layout

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
│   │   ├── messages/    # Direct message conversations (v1 + v2)
│   │   ├── keys/        # Signal Protocol key server
│   │   ├── devices/     # Multi-device registration and trust
│   │   ├── canvas/      # Live Canvas (music, polls, clips)
│   │   ├── explore/     # Discovery, search, trending tags
│   │   ├── parental/    # COPPA parental controls
│   │   ├── media/       # R2 presigned upload URLs
│   │   ├── emojis/      # Custom server emoji (WebP conversion)
│   │   ├── notifications/ # FCM push dispatch
│   │   ├── screentime/  # Screen time reporting
│   │   ├── bots/        # Bot application management
│   │   ├── discord/     # Discord profile import
│   │   ├── premium/     # Stripe webhooks + promo codes
│   │   └── support/     # Support tickets → HubSpot CRM
│   ├── migrations/      # sqlx SQL migrations (26 total)
│   └── Dockerfile       # cargo-chef multi-stage build
│
├── frontend/            # SvelteKit app (PWA + Tauri + Capacitor)
│   ├── src/
│   │   ├── lib/
│   │   │   ├── signal/  # X3DH, double ratchet, sender keys, keystore
│   │   │   ├── stores/  # Svelte stores (auth, ws, servers, canvas…)
│   │   │   ├── api/     # Typed HTTP client
│   │   │   └── components/
│   │   └── routes/
│   │       ├── (auth)/  # Login, register, OAuth callback
│   │       └── (app)/   # App shell (servers, DMs, explore, settings…)
│   ├── tests/           # Playwright E2E specs
│   ├── src-tauri/       # Tauri v2 shell
│   ├── ios/             # Capacitor iOS platform
│   └── android/         # Capacitor Android platform
│
├── marketing/           # Astro marketing site → Cloudflare Pages
├── scripts/             # Utility scripts
├── docker-compose.yml   # Local PostgreSQL
└── Makefile             # Common dev tasks
```

---

## Quick start

### Prerequisites

- Rust 1.85+ — [rustup.rs](https://rustup.rs)
- Node.js 20+ — [nodejs.org](https://nodejs.org)
- PostgreSQL 16+ (or Docker)
- `cargo install sqlx-cli --no-default-features --features postgres`

### 1. Clone and configure

```bash
git clone https://github.com/NikheelR97/Yapper.git
cd Yapper
cp backend/.env.example backend/.env   # then edit DATABASE_URL etc.
```

### 2. Generate JWT keys

```bash
mkdir -p backend/secrets
openssl genrsa -out backend/secrets/jwt_private.pem 2048
openssl rsa -in backend/secrets/jwt_private.pem -pubout -out backend/secrets/jwt_public.pem
```

### 3. Start PostgreSQL and run migrations

```bash
make db-up      # starts Docker postgres on :5432
make migrate    # applies all sqlx migrations
```

### 4. Start the backend

```bash
make dev-backend   # cargo watch → localhost:8080
```

### 5. Start the frontend

```bash
cd frontend && npm install
make dev-frontend  # SvelteKit dev server → localhost:5173
```

### 6. Optional — desktop app

```bash
make dev-tauri   # Tauri window with hot-reload
```

See the [Getting Started](https://github.com/NikheelR97/Yapper/wiki/Getting-Started) wiki page for a full walkthrough.

---

## Common make targets

| Command | What it does |
|---------|-------------|
| `make dev` | Docker postgres + backend hot-reload |
| `make dev-frontend` | SvelteKit dev server |
| `make dev-tauri` | Tauri desktop dev |
| `make migrate` | Apply pending DB migrations |
| `make sqlx-prepare` | Regenerate sqlx offline query cache |
| `make test` | Backend + frontend unit tests |
| `make lint` | clippy + eslint |
| `make fmt` | cargo fmt + prettier |

---

## Documentation

Full documentation is available in the [GitHub Wiki](https://github.com/NikheelR97/Yapper/wiki).

| Page | Description |
|------|-------------|
| [Architecture](https://github.com/NikheelR97/Yapper/wiki/Architecture) | System diagram, tech stack, real-time hub design |
| [Getting Started](https://github.com/NikheelR97/Yapper/wiki/Getting-Started) | Full local development setup |
| [Backend Development](https://github.com/NikheelR97/Yapper/wiki/Backend-Development) | Module guide, patterns, conventions |
| [Frontend Development](https://github.com/NikheelR97/Yapper/wiki/Frontend-Development) | SvelteKit + Tauri + Capacitor guide |
| [Database](https://github.com/NikheelR97/Yapper/wiki/Database) | Schema, all 26 migrations, sqlx cache workflow |
| [API Reference](https://github.com/NikheelR97/Yapper/wiki/API-Reference) | All HTTP endpoints |
| [E2EE Implementation](https://github.com/NikheelR97/Yapper/wiki/E2EE-Implementation) | X3DH, double ratchet, sender keys, media encryption |
| [Deployment](https://github.com/NikheelR97/Yapper/wiki/Deployment) | Fly.io, Cloudflare Pages, CI/CD pipeline |
| [Contributing](https://github.com/NikheelR97/Yapper/wiki/Contributing) | How to contribute |
| [Security](https://github.com/NikheelR97/Yapper/wiki/Security) | Threat model and vulnerability disclosure |

---

## Security model

- **E2EE is inviolable** — the server cannot read message content
- **Parental controls operate on metadata only** — parents see who their child is communicating with, never message content
- CSRF double-submit cookie on all mutating endpoints
- Argon2id (memory-hard) for password hashing
- JWT RS256 with 15-minute access tokens + HttpOnly refresh cookies
- Rate limiting: 100 req/min per IP + 5 msg/sec per user (WebSocket)
- All secrets in Fly.io secret store — never committed to git

See [Security](https://github.com/NikheelR97/Yapper/wiki/Security) for the full threat model and responsible disclosure process.

---

## Project status

All core sprints complete. Currently in launch phase.

| Sprint | Theme | Status |
|--------|-------|--------|
| S0 — Setup + Marketing | Marketing site, wishlist | ✅ |
| S1 — Scaffolding | Repo, CI, DB schema | ✅ |
| S2 — Auth | Register, login, OAuth, JWT | ✅ |
| S3 — E2EE Core | Signal Protocol, DMs | ✅ |
| S4 — Servers & Channels | Group E2EE, invite links | ✅ |
| S5 — Real-Time | Typing, read receipts, presence | ✅ |
| S6 — Live Canvas + Explore | Canvas widgets, discovery | ✅ |
| S7 — Profiles + Parental | Social graph, COPPA controls | ✅ |
| S8 — Screen Time + Discord | OS screen time, Discord import | ✅ |
| S9 — Emojis + Settings | Custom emoji, all settings | ✅ |
| S10 — Desktop Polish | Tauri, auto-updater, security audit | ✅ |
| S11 — Premium + Launch | Stripe, Sentry, production deploy | ✅ |
| S12 — Multi-Device E2EE | Per-device keys, trust workflow | ✅ |
| S13 — Support + Infra | Support tickets, build pipeline | ✅ |

---

## License

Proprietary — all rights reserved. Not open source.
