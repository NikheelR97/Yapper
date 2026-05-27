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

GitHub Actions does not currently auto-promote production deploys during the stabilization sprint. Treat Fly.io and Cloudflare Pages promotions as separate operational steps until the release gate is restored.

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

Production deploy automation is paused in GitHub Actions during stabilization. Use `flyctl deploy` manually only after the release gate is reinstated.

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

### Health probe

The `/health` endpoint executes `SELECT 1` against the Neon pool with a 2 s timeout. A 200 means both the VM and the database are reachable; a 503 means the pool is dead even though the VM is up. Fly.io's HTTP health check should be pointed at `/health` so a Neon outage triggers a probe failure rather than silent 500s on application requests.

```bash
curl -fsS https://api.yapperhq.com/health
# {"db":"ok","ok":true}
```

---

## Emergency operations

### Scale up (memory pressure or CPU saturation)

The MVP runs a single `shared-cpu-1x` machine with 256 MB RAM. To scale vertically:

```bash
# Larger VM (preserves the single-instance topology — safe for the hub):
flyctl scale vm shared-cpu-2x --vm-memory 512 -a yapper-api

# Or, only during a rolling redeploy, run two machines briefly:
flyctl scale count 2 --region jnb -a yapper-api
flyctl machine list -a yapper-api          # confirm both are healthy
flyctl scale count 1 --region jnb -a yapper-api   # back to single-instance
```

> **WARNING — multi-machine is not yet safe.** The WebSocket hub state lives in process memory (`Arc<DashMap>` in `backend/src/hub.rs`). Two concurrent machines will not see each other's connected users, so presence, typing indicators, and channel fan-out will be split-brained until the hub-sharding work lands. Only use `count 2` during a deliberately short rolling deploy window. For sustained higher load, scale **up** (`shared-cpu-2x` or larger), not **out**.

### VM unresponsive or stuck

```bash
flyctl machine list -a yapper-api
flyctl machine restart <machine-id> -a yapper-api
```

Clients reconnect automatically and undelivered messages replay from PostgreSQL on the next WebSocket handshake. Acceptable downtime window: ~10 seconds.

### Neon storage approaching the 500 MB free-tier hard cap

The release gate requires automated `pg_database_size()` monitoring (audit ref MED-002). Until that is in place, check manually before any large data import:

```bash
psql "$NEON_DIRECT_URL" -c "SELECT pg_size_pretty(pg_database_size(current_database()));"
```

If above 400 MB, schedule an upgrade to Neon Pro from the Neon dashboard ($19/mo, no downtime). If writes are already failing because the cap was hit:

```bash
# Tail backend logs for the error pattern
flyctl logs -a yapper-api | grep -iE 'disk full|storage|insert.*failed'
# Then upgrade Neon, then restart the VM to drop any cached pool errors:
flyctl machine restart <machine-id> -a yapper-api
```

### Rollback to a previous release

```bash
flyctl releases -a yapper-api                   # find the last known-good SHA
flyctl image show -a yapper-api                 # confirm current image
flyctl deploy --image registry.fly.io/yapper-api:<git-sha> -a yapper-api
```

The release gate requires that the rollback target is from the **same** migration generation as the current schema. If a rollback would require reverting a migration, do **not** rollback the VM alone — open a manual incident and revert the migration first. See "Database migrations in production" above.

### Public incident communication

If user-facing degradation lasts more than 5 minutes, post to status.yapperhq.com (Cloudflare Page) and the @yapperhq social accounts. Do not include user-identifying detail or root-cause speculation in the public post.

---

## Staging environment

### Current state (2026-05-27)

Staging is **referenced but not provisioned**. The E2E nightly workflow defaults to `https://staging.yapperhq.com` and `https://staging-api.yapperhq.com`, but:

- There is no `yapper-api-staging` Fly app — only `yapper-api` exists.
- `staging-api.yapperhq.com` returns Cloudflare `HTTP 502` because the Fly proxy has no cert/app matching that hostname (`yapper-api` only has the `api.yapperhq.com` cert). Fly logs show `proxy [error] client problem: invalid authority` for staging requests.
- As a result the nightly's `probe-auth` job has been reporting `edge-blocked` and the actual Playwright shards have been **skipped on every scheduled run** for at least the past week, while the workflow still exits green.

The PR smoke suite (`e2e-pr-smoke.yml`) runs against a different target and is not affected; it remains the working E2E signal for PRs.

### Goals

Full isolation from production. Staging exists to validate a release candidate before promoting to production, so it must not share secrets, database, or media storage with production. Cost target: stay on Fly + Neon + R2 + Cloudflare free tiers.

### Provisioning runbook

Run these steps from a workstation that has `flyctl auth login` for the same Fly organization as `yapper-api`, and the same Cloudflare / Neon credentials used for production. Each step is independently reversible.

1. **Create a Neon staging branch.**
   Use the Neon console (Branches → "Create branch" from the `main` branch) or `neonctl branches create --name staging`. This gives staging its own data plane while sharing the same Neon project quota. Capture the staging branch connection string for `DATABASE_URL`.

2. **Provision a staging R2 prefix.**
   Cheapest path: reuse the existing `yapper-media` bucket with the env-aware key prefix the backend already supports (`APP_ENV=staging` ⇒ `staging/<...>`). Alternative: create a separate `yapper-media-staging` bucket if isolation of object lifecycle and metrics is required. The R2 API token can be re-used or scoped narrower. Capture `R2_BUCKET_NAME` and `R2_PUBLIC_URL` (staging).

3. **Generate a staging JWT keypair.**
   Do not reuse the production key material. Generate fresh RS256 keys:
   ```bash
   openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out jwt_private.pem
   openssl rsa -in jwt_private.pem -pubout -out jwt_public.pem
   ```
   Keep both files out of the repo; they will be loaded into Fly secrets only.

4. **Create the Fly app `yapper-api-staging`.**
   From `backend/`:
   ```bash
   flyctl apps create yapper-api-staging
   flyctl config save -a yapper-api -o fly.staging.toml
   # edit fly.staging.toml: change `app = "yapper-api"` -> `app = "yapper-api-staging"`,
   # keep primary_region = "jnb", keep the same vm/concurrency/http_service settings.
   flyctl deploy -a yapper-api-staging -c fly.staging.toml
   ```
   The deploy will fail the health check until secrets are set in the next step; that is expected.

5. **Set secrets on `yapper-api-staging`.**
   Mirror the production secret names listed by `flyctl secrets list -a yapper-api`, with all values pointing at staging resources:
   ```bash
   flyctl secrets set -a yapper-api-staging \
     APP_ENV="staging" \
     DATABASE_URL="<neon staging branch URL>" \
     JWT_PRIVATE_KEY="$(cat jwt_private.pem)" \
     JWT_PUBLIC_KEY="$(cat jwt_public.pem)" \
     API_BASE_URL="https://staging-api.yapperhq.com" \
     FRONTEND_URL="https://staging.yapperhq.com" \
     CORS_ORIGINS="https://staging.yapperhq.com,http://tauri.localhost,capacitor://localhost" \
     R2_ACCOUNT_ID="<same as prod or scoped staging account>" \
     R2_ACCESS_KEY_ID="<scoped staging token>" \
     R2_SECRET_ACCESS_KEY="<scoped staging token>" \
     R2_BUCKET_NAME="<staging bucket or shared bucket name>" \
     R2_PUBLIC_URL="<staging media URL>" \
     EMAIL_FROM="staging@yapperhq.com" \
     SENTRY_DSN="<staging Sentry project DSN, or empty>" \
     FCM_SERVICE_ACCOUNT_JSON="<staging Firebase service account JSON>" \
     DISCORD_CLIENT_ID="<staging Discord OAuth app>" \
     DISCORD_CLIENT_SECRET="<staging Discord OAuth app>" \
     GOOGLE_CLIENT_ID="<staging Google OAuth client>" \
     GOOGLE_CLIENT_SECRET="<staging Google OAuth client>" \
     HUBSPOT_ACCESS_TOKEN="<test HubSpot token or empty>"
   ```
   OAuth credentials must be separate apps because the production OAuth apps' redirect URIs are scoped to `api.yapperhq.com`. Without staging-specific OAuth apps, login-via-OAuth tests cannot pass on staging.

6. **Run database migrations against the staging branch.**
   ```bash
   DATABASE_URL="<neon staging branch URL>" make migrate
   ```

7. **Attach the custom domain.**
   ```bash
   flyctl certs add staging-api.yapperhq.com -a yapper-api-staging
   ```
   In Cloudflare DNS, ensure `staging-api.yapperhq.com` is a proxied CNAME to `yapper-api-staging.fly.dev` (orange-cloud), and that the SSL/TLS mode is "Full (strict)" — the same configuration used for `api.yapperhq.com`.

8. **Verify backend.**
   ```bash
   curl -i https://staging-api.yapperhq.com/health    # expect 200, body indicates db:true
   ```
   If the response is still `502`, run `flyctl certs check staging-api.yapperhq.com -a yapper-api-staging` to confirm cert issuance and DNS pointing.

9. **Provision the staging frontend on Cloudflare Pages.**
   Either (a) create a second Pages project `yapper-app-staging` pointed at the same repo's `frontend/` directory with the `staging` branch as the production branch, or (b) reuse the existing `yapper-app` project's preview deployments (`*.pages.dev`) and add a custom domain alias `staging.yapperhq.com` pointing at a specific preview environment. Option (a) is simpler operationally; option (b) avoids a second project. Either way, the Pages project's environment variables must include `VITE_API_URL=https://staging-api.yapperhq.com`.

10. **Verify staging end-to-end.**
    ```bash
    curl -sI https://staging.yapperhq.com/ | head -5     # expect 200 + Cloudflare headers
    curl -s https://staging-api.yapperhq.com/health      # expect db-healthy JSON
    ```
    Then trigger the nightly manually with `gh workflow run e2e-nightly.yml` and confirm the `probe-auth` step now reports `reachable=true` and the Playwright shards actually run instead of being skipped.

11. **Update the E2E secrets if needed.**
    The `E2E_EMAIL` / `E2E_PASSWORD` GitHub repo secrets must correspond to test accounts that exist in the staging database. If they currently point at production, create matching accounts on staging (one trusted-device user, one secondary) and either rotate the secrets or seed staging with the same credentials.

### Future cleanup

Once staging is provisioned and the nightly is producing real signal, consider:

- Replacing the workflow's `edge-blocked` graceful-skip with a hard failure — silent skips were the original blind spot.
- Documenting staging-vs-prod secret rotation cadence (quarterly) in `dev docs/SECURITY_AUDIT.md`.
- Re-running the release gate checklist against the staging environment; it has only ever been validated against production today.

---

## Frontend — Cloudflare Pages

The frontend is deployed automatically by Cloudflare Pages on push to `main`. That provider-side deploy is outside the GitHub release gate and should be treated as a separate production promotion path.

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

Marketing Pages deploys are also provider-managed and are not part of the paused GitHub production promotion pipeline.

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
