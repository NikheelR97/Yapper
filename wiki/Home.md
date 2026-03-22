# Yapper — Wiki

Yapper is an open-source, end-to-end encrypted real-time chat platform. It supports direct messaging, group servers with channels, live audio/video clips, a canvas, parental controls, and cross-platform clients.

## Quick links

| Page | Description |
|------|-------------|
| [Architecture](Architecture) | System design, tech stack, data flow |
| [Getting Started](Getting-Started) | Set up the development environment |
| [Backend Development](Backend-Development) | Rust/Axum API guide |
| [Frontend Development](Frontend-Development) | SvelteKit + Tauri + Capacitor guide |
| [Database](Database) | Schema overview and migration guide |
| [API Reference](API-Reference) | All HTTP endpoints |
| [E2EE Implementation](E2EE-Implementation) | Signal-style encryption design |
| [Deployment](Deployment) | Fly.io, Cloudflare Pages, CI/CD |
| [Contributing](Contributing) | How to contribute |
| [Security](Security) | Security model and disclosure policy |

## What is Yapper?

- **Real-time messaging** — WebSocket hub with per-user presence, typing indicators and read receipts
- **End-to-end encryption** — X3DH key agreement + double ratchet (AES-256-GCM) for DMs; Sender Keys for group channels
- **Multi-platform** — Web PWA · Windows/macOS/Linux desktop (Tauri v2) · iOS/Android (Capacitor)
- **Parental controls** — COPPA-compliant child accounts; parent approves friend requests and server joins
- **$0/month infra** — Fly.io (backend) · Neon PostgreSQL · Cloudflare Pages (frontend) · Cloudflare R2 (media)

## Repository layout

```
yapper/
├── backend/          # Rust + Axum API server
│   ├── src/          # Application source
│   ├── migrations/   # sqlx SQL migrations (30 total)
│   └── Dockerfile    # cargo-chef multi-stage build
├── frontend/         # SvelteKit web + Tauri desktop + Capacitor mobile
│   ├── src/          # SvelteKit app
│   ├── src-tauri/    # Tauri v2 shell
│   ├── ios/          # Capacitor iOS platform
│   ├── android/      # Capacitor Android platform
│   └── tests/        # Playwright E2E tests
├── marketing/        # Astro marketing site (yapperhq.com)
├── scripts/          # Utility scripts
├── dev docs/         # Internal developer documents (gitignored from CI)
└── .github/
    └── workflows/    # CI · E2E nightly · desktop release · dependabot
```

## Live environments

| Environment | URL |
|-------------|-----|
| App | https://app.yapperhq.com |
| API | https://api.yapperhq.com |
| Marketing | https://yapperhq.com |
| API health | https://api.yapperhq.com/health |
