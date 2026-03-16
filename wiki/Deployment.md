# Deployment

## Overview

| Component | Platform | URL |
|-----------|----------|-----|
| Backend API | Fly.io (jnb region) | https://api.yapperhq.com |
| Frontend app | Cloudflare Pages | https://app.yapperhq.com |
| Marketing site | Cloudflare Pages | https://yapperhq.com |
| Database | Neon (PostgreSQL) | Internal — not public |
| Media storage | Cloudflare R2 | https://pub-…r2.dev |

---

## CI/CD pipeline

Every push to `main` triggers `.github/workflows/ci.yml`:

```
push to main
    │
    ├── Backend (Rust) ──────────────────────────────────────────┐
    │   fmt check → clippy → tests → audit → sqlx cache         │
    │                                                            ▼
    │                                                   Deploy backend to Fly.io
    │                                                   (build image in GHA → push to
    │                                                    registry.fly.io → flyctl deploy)
    │
    ├── Frontend (SvelteKit) ──────────────────────────────────┐
    │   type check → unit tests → audit → build                │
    │                                                           ▼
    │                                                  Deploy frontend to Cloudflare Pages
    │
    └── Marketing (Astro) ─────────────────────────────────────┐
        build                                                   │
                                                                ▼
                                                       Deploy marketing to Cloudflare Pages
```

Backend and frontend deploys run in parallel (independent `needs:` chains).

---

## Backend — Fly.io

### Build pipeline

The Docker image is built in GitHub Actions using `cargo-chef` for layer caching:

```
Stage 1: lukemathwalker/cargo-chef (planner)
  → cargo chef prepare --recipe-path recipe.json

Stage 2: lukemathwalker/cargo-chef (builder)
  → cargo chef cook --release    ← cached when only source changes
  → cargo build --release

Stage 3: alpine:3.19 (runtime)
  → copy binary + migrations
  → run as non-root 'yapper' user
```

The `cargo chef cook` layer (~400–500 MB) is cached in GitHub Actions cache (`type=gha,mode=max`). On a source-only change, this step is skipped — reducing build time from ~15 min to ~90 s.

### Deploying manually

```bash
# Build and push image
docker build -t registry.fly.io/yapper-api:latest ./backend
docker push registry.fly.io/yapper-api:latest

# Deploy without rebuilding
flyctl deploy --image registry.fly.io/yapper-api:latest -a yapper-api
```

### Rolling back

```bash
# List recent image SHAs
flyctl releases -a yapper-api

# Rollback to a specific commit SHA
flyctl deploy --image registry.fly.io/yapper-api:<git-sha> -a yapper-api
```

### Secrets management

All secrets are stored in Fly.io (never in the repo):

```bash
flyctl secrets set KEY=value -a yapper-api
flyctl secrets list -a yapper-api
```

Required secrets: `DATABASE_URL`, `JWT_PRIVATE_KEY`, `JWT_PUBLIC_KEY`, `RESEND_API_KEY`, `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `R2_*` (5 vars), `CORS_ORIGINS`, `SENTRY_DSN`, `HUBSPOT_ACCESS_TOKEN`, `FCM_SERVICE_ACCOUNT_JSON`.

### Machine configuration

```toml
# backend/fly.toml
primary_region = "jnb"   # Johannesburg
min_machines_running = 1
auto_stop_machines = false

[[vm]]
  memory = "256mb"
  cpu_kind = "shared"
  cpus = 1
```

---

## Frontend — Cloudflare Pages

Deployed via Wrangler:

```bash
cd frontend
npm run build
npx wrangler pages deploy build --project-name yapper-app --branch main
```

Environment variables baked in at build time:

```
VITE_API_URL=https://api.yapperhq.com
VITE_WS_URL=wss://api.yapperhq.com/ws
VITE_FIREBASE_VAPID_KEY=<public key>
VITE_SENTRY_DSN=<dsn>
```

---

## Marketing — Cloudflare Pages

```bash
cd marketing
npm run build
npx wrangler pages deploy dist --project-name yapper-marketing --branch main
```

---

## Database migrations in production

Migrations run automatically when the backend starts. No manual step required. To run manually against Neon:

```bash
DATABASE_URL="postgres://…neon-direct-endpoint…" sqlx migrate run
```

Use the **direct** (non-pooler) endpoint — the pooler endpoint uses PgBouncer which doesn't support the `SET` commands sqlx uses.

---

## E2E nightly tests

`.github/workflows/e2e-nightly.yml` runs at 02:00 SAST (00:00 UTC) against production. It can also be triggered manually:

```bash
gh workflow run e2e-nightly.yml --field base_url=https://app.yapperhq.com
```

Required GitHub Secrets: `E2E_EMAIL`, `E2E_PASSWORD`, `E2E_EMAIL_2`, `E2E_PASSWORD_2`.

---

## Desktop releases

`.github/workflows/release-desktop.yml` triggers on version tags (`v*.*.*`):

```bash
git tag v1.0.0
git push origin v1.0.0
```

Produces a draft GitHub Release with:
- Windows: `Yapper_x.x.x_x64-setup.exe` (NSIS installer)
- macOS: `Yapper_x.x.x_universal.dmg` (requires Mac runner + Apple signing secrets)
- Linux: `Yapper_x.x.x_amd64.AppImage` + `.deb`

Required secrets for signing: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Generate with:

```bash
cargo tauri signer generate -w ~/.tauri/yapper.key
```
