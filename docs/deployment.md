# Deployment Guide

## Infrastructure Overview

| Component | Platform | URL |
|-----------|----------|-----|
| Backend API | Fly.io (Rust binary) | api.yapperhq.com |
| Frontend app | Cloudflare Pages | app.yapperhq.com |
| Marketing site | Cloudflare Pages | yapperhq.com |
| Database | Neon (PostgreSQL) | — |
| Media storage | Cloudflare R2 | media.yapperhq.com |
| Wishlist API | Cloudflare Worker | yapperhq.com/api/* |
| Email | Resend | — |
| Push | Firebase (FCM) | — |

All production infra runs on free tiers. Monthly cost: $0 (plus ~$10/year domain).

---

## Backend — Fly.io

### App

- **Name:** `yapper-api`
- **Region:** `jnb` (Johannesburg)
- **CLI:** `flyctl` (install: `curl -L https://fly.io/install.sh | sh`)

### First-time deploy

```bash
cd backend
flyctl auth login
flyctl launch --no-deploy   # creates fly.toml if not present
flyctl secrets set \
  DATABASE_URL="postgres://..." \
  JWT_PRIVATE_KEY="$(cat secrets/jwt_private.pem)" \
  JWT_PUBLIC_KEY="$(cat secrets/jwt_public.pem)" \
  RESEND_API_KEY="re_..." \
  DISCORD_CLIENT_ID="..." \
  DISCORD_CLIENT_SECRET="..." \
  GOOGLE_CLIENT_ID="..." \
  GOOGLE_CLIENT_SECRET="..." \
  R2_ACCOUNT_ID="..." \
  R2_ACCESS_KEY_ID="..." \
  R2_SECRET_ACCESS_KEY="..." \
  R2_BUCKET_NAME="yapper-media" \
  R2_ENDPOINT="https://....r2.cloudflarestorage.com" \
  R2_PUBLIC_URL="https://media.yapperhq.com" \
  FCM_SERVICE_ACCOUNT_JSON="$(cat secrets/firebase-service-account.json)" \
  CORS_ORIGINS="https://app.yapperhq.com,http://tauri.localhost,capacitor://localhost" \
  APP_ENV="production"
flyctl deploy
```

### Subsequent deploys

CI runs `flyctl deploy` directly in the `deploy-backend` job — no separate Docker push step is needed. Fly.io handles the remote build from the Dockerfile.

```bash
# Manual deploy (same as what CI does):
make deploy-backend   # cd backend && flyctl deploy
```

### Environment variables (production vs local)

In production, JWT keys are passed as the full PEM content in `JWT_PRIVATE_KEY` and `JWT_PUBLIC_KEY` env vars (no file paths). The backend loads whichever is set.

### Database migrations in production

```bash
# From local machine with DATABASE_URL pointed at Neon direct endpoint:
make migrate
```

Use the **non-pooler** Neon endpoint (`ep-broad-block-a9johezc.gwc.azure.neon.tech`) for migrations and `cargo sqlx prepare`. The pooler endpoint (PgBouncer) does not support prepared statements.

### Viewing logs

```bash
flyctl logs -a yapper-api
flyctl logs -a yapper-api --tail   # live tail
```

### SSH into the VM

```bash
flyctl ssh console -a yapper-api
```

---

## Frontend — Cloudflare Pages

The frontend is deployed automatically via GitHub Actions on push to `main`.

**Project:** `yapper-app` → `app.yapperhq.com`

### Manual deploy

```bash
cd frontend
npm run build        # SvelteKit static build
npx wrangler pages deploy build --project-name yapper-app
```

### Environment variables (Cloudflare Pages dashboard)

| Variable | Value |
|----------|-------|
| `VITE_API_URL` | `https://api.yapperhq.com` |
| `VITE_WS_URL` | `wss://api.yapperhq.com/ws` |
| `VITE_FIREBASE_VAPID_KEY` | FCM VAPID public key |
| `PUBLIC_FIREBASE_CONFIG` | JSON string of Firebase web config |

---

## Marketing Site — Cloudflare Pages

**Project:** `yapper-marketing` → `yapperhq.com`

```bash
make deploy-marketing   # cd marketing && npm run build && wrangler pages deploy dist
```

### Wishlist Worker

```bash
make deploy-worker   # cd marketing && wrangler deploy
```

---

## Database — Neon

**Project:** `neondb`

| Endpoint type | URL |
|---------------|-----|
| Pooler (application) | `ep-broad-block-a9johezc-pooler.gwc.azure.neon.tech` |
| Direct (migrations/sqlx) | `ep-broad-block-a9johezc.gwc.azure.neon.tech` |

**Connection string format:**
```
postgres://neondb_owner:<password>@<endpoint>/neondb?sslmode=require
```

The `DATABASE_URL` in `backend/.env` (and Fly.io secret) should use the **pooler** endpoint for runtime. Use the direct endpoint only for:
- `sqlx migrate run`
- `cargo sqlx prepare`

---

## Media — Cloudflare R2

**Bucket:** `yapper-media`

R2 CORS configuration (set via Cloudflare dashboard):
```json
[{
  "AllowedOrigins": ["https://app.yapperhq.com", "http://localhost:5173"],
  "AllowedMethods": ["GET", "PUT"],
  "AllowedHeaders": ["*"],
  "MaxAgeSeconds": 3600
}]
```

Media files are **client-side AES-256-GCM encrypted** before upload. R2 stores ciphertext only.

---

## Firebase / FCM

**Project:** `yapper-41f63`

Service account JSON is stored at `backend/secrets/firebase-service-account.json` (gitignored) and in the `FCM_SERVICE_ACCOUNT_JSON` Fly.io secret.

VAPID public key (for web push):
```
BOrz15S2L_kg0_R5Tam_gbtO_Zf3LzTVFwzQGbCsDFLZgi6fG5DUk3KjIMLs4N3oGeaGkJcbxfeBlHGzDF5k8Cc
```

---

## DNS (Cloudflare — yapperhq.com)

| Record | Type | Target |
|--------|------|--------|
| `@` | CNAME | `yapper-marketing.pages.dev` |
| `www` | CNAME | `yapper-marketing.pages.dev` |
| `app` | CNAME | `yapper-app.pages.dev` |
| `api` | CNAME | `yapper-api.fly.dev` (Fly.io sets this) |
| `media` | CNAME | R2 public bucket endpoint |

---

## Tauri Desktop Build

```bash
cd frontend
npm run tauri build
# Output: src-tauri/target/release/bundle/
#   Windows: .msi + .exe
#   macOS:   .dmg + .app
#   Linux:   .AppImage + .deb
```

Code signing (required for distribution):
- **Windows:** Signtool + EV certificate (or use Tauri's updater with a self-signed cert for internal use)
- **macOS:** Apple Developer ID certificate + notarization (`xcrun notarytool`)
- **Linux:** No signing required for AppImage

---

## Capacitor Mobile Build

### iOS (requires macOS + Xcode)

```bash
cd frontend
npm run build
npx cap sync ios
npx cap open ios   # opens Xcode
# Build + Archive in Xcode, then upload to App Store Connect
```

Requires:
- Apple Developer Program ($99/year — only needed at submission)
- CocoaPods (`pod install` in `frontend/ios/App/`)
- `GoogleService-Info.plist` from Firebase console → `frontend/ios/App/App/`

### Android

```bash
cd frontend
npm run build
npx cap sync android
npx cap open android   # opens Android Studio
# Build → Generate Signed Bundle/APK
```

Requires:
- `google-services.json` from Firebase console → `frontend/android/app/`
- Google Play Developer account ($25 one-time — only at submission)

---

## CI/CD

GitHub Actions workflows live in `.github/workflows/`.

Triggers:
- **Push to `main`:** Run `cargo check`, `cargo clippy`, frontend type check, deploy if all pass
- **Pull request:** Run checks only, no deploy

The `deploy-backend` job runs `flyctl deploy` directly (no `docker/build-push-action` or manual push to `registry.fly.io`). Fly.io performs the remote Docker build from the committed Dockerfile and deploys the image in one step.

> **Note:** `CORS_ORIGINS` must include `http://tauri.localhost` for Tauri v2 desktop. Tauri v2 sends `Origin: http://tauri.localhost` (not `tauri://localhost` from v1).

Secrets needed in GitHub repository settings:
- `FLY_API_TOKEN` — from `flyctl tokens create deploy`
- `CF_API_TOKEN` — Cloudflare Pages deploy token
