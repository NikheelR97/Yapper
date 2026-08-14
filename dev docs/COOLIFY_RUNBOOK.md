# Yapper — Coolify Self-Hosting Runbook

**Created:** 2026-07-20
**Scope:** migrating the Yapper backend + PostgreSQL from Fly.io/Neon to self-hosted **Coolify** on the homelab box (20 cores / 128 GB / 2 TB NVMe).
**Status:** prep — not yet executed. Tracked as S18b in [`LAUNCH_SPRINT_PLAN.md`](LAUNCH_SPRINT_PLAN.md).

---

## 1. What moves, and what deliberately does not

| Component | Destination | Rationale |
|---|---|---|
| **Rust backend** | **Coolify** | Already Dockerized; Coolify deploys the existing `backend/Dockerfile` directly. |
| **PostgreSQL** | **Coolify** | The real win: removes Neon's 0.5 GB storage cap *and* its 500 ms–2 s cold starts. |
| Frontend app | **stays on Cloudflare Pages** | Free, global CDN, DDoS protection. A residential connection cannot match it. |
| Marketing site | **stays on Cloudflare Pages** | Same, plus the wishlist Worker is coupled to Cloudflare D1/KV — migrating is pure busywork. |
| Media (R2) | **stays on Cloudflare R2** | Free to 10 GB, no egress fees, already S3-endpoint-configurable. |

> Moving media to MinIO on Coolify is a pure env-var swap (`R2_*` → MinIO endpoint) available later if you ever want full consolidation. Not now — there's no problem to solve.

**Accepted risk:** a single homelab box is a single point of failure (power, home internet, no redundancy). Fine for beta. The exit is cheap precisely *because* the frontend and media stay on portable free tiers.

---

## 2. Deploy the backend

Coolify → new resource → **Dockerfile** application, pointed at the repo with build context `backend/`.

The Dockerfile is multi-stage (cargo-chef → Alpine runtime), runs as a non-root `yapper` user, and builds with `SQLX_OFFLINE=true` against the committed `.sqlx` cache — no database is needed at build time.

Health check: `GET /health` returns `{"status":"ok","db":true}` and **503 if the DB pool is unreachable** (2 s timeout). Point Coolify's healthcheck at it.

---

## 3. Provision PostgreSQL + run migrations

1. Create a Coolify PostgreSQL service (v16 to match `docker-compose.yml`).
2. Point the backend's `DATABASE_URL` at it.
3. Apply the 38 migrations:
   ```bash
   sqlx migrate run   # from backend/, with DATABASE_URL set
   ```
   Migrations are timestamp-ordered and sorted by full filename — the three duplicated sequence numbers (000019/000020/000031) are a cosmetic naming quirk only and apply in the correct order.

---

## 4. Environment / secrets checklist

Derived from `env::var(...)` across `backend/src` (39 vars). Set these as Coolify secrets — never in the repo.

### Required

| Var | Notes |
|---|---|
| `DATABASE_URL` | Coolify Postgres connection string. |
| `JWT_PRIVATE_KEY` *or* `JWT_PRIVATE_KEY_PATH` | RS256 private key (PEM). Never leaves the API service. |
| `JWT_PUBLIC_KEY` *or* `JWT_PUBLIC_KEY_PATH` | RS256 public key. |
| `JWT_KEY_ID` | `kid` header, for key rotation. |
| `PORT` / `HOST` | Bind address for the container. |
| `FRONTEND_URL` | `https://app.yapperhq.com` — used for CORS + email links. |
| `CORS_ORIGINS` | Allowlist — see §6. |
| `COOKIE_SECURE` | **`true`** in production (refresh cookie is `SameSite=None`). |
| `TRUSTED_PROXY_IPS` | **Critical behind Cloudflare Tunnel** — see §5. |
| `BASE_URL` / `API_BASE_URL` | `https://api.yapperhq.com`. |
| `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET_NAME`, `R2_PUBLIC_URL` | Media storage (stays on R2). |
| `RESEND_API_KEY` | Email verification + password reset. |

### Optional (feature-gated; absent = feature disabled, no crash)

| Var(s) | Feature |
|---|---|
| `DISCORD_CLIENT_ID` / `_SECRET` | Discord OAuth + profile/bot import |
| `GOOGLE_CLIENT_ID` / `_SECRET` | Google OAuth |
| `APPLE_CLIENT_ID`, `APPLE_TEAM_ID`, `APPLE_KEY_ID`, `APPLE_PRIVATE_KEY` | Apple Sign-In (UI currently disabled) |
| `FCM_SERVICE_ACCOUNT_JSON` | Push notifications |
| `SENTRY_DSN`, `SENTRY_ENVIRONMENT` | Error monitoring |
| `HUBSPOT_ACCESS_TOKEN`, `HUBSPOT_CLIENT_SECRET`, `HUBSPOT_WEBHOOK_URL`, `HUBSPOT_INCLUDE_PII` | Support tickets |
| `STRIPE_WEBHOOK_SECRET`, `GOPRO_PROMO_CODES` | Premium |
| `EMAIL_FROM` | Defaults to `Yapper <hello@yapperhq.com>` |

`FLY_APP_NAME` becomes irrelevant once off Fly.io.

---

## 5. Expose via Cloudflare Tunnel

Use `cloudflared`, **not** port-forwarding: no inbound ports open, residential IP stays hidden, TLS terminates at Cloudflare's edge, and the domain already lives there.

1. Create a tunnel; route `api.yapperhq.com` → `http://<backend-container>:<PORT>`.
2. **WebSockets must be enabled** — the entire realtime hub (messaging, typing, presence, canvas) runs over WSS. Verify a `wss://api.yapperhq.com/ws` upgrade succeeds before cutover.
3. **Set `TRUSTED_PROXY_IPS` to Cloudflare's egress ranges.** `extract_ip()` only honours `X-Forwarded-For`/`CF-Connecting-IP` from trusted sources (security fix H-04). Get this wrong and either every request looks like it came from one IP (rate limiting collapses to a global bucket) or client IPs are spoofable.

---

## 6. Cutover config changes

The app stays **cross-origin** (frontend on Pages, API on the homelab), so the existing cookie/CSRF model is unchanged — do **not** "simplify" it:

- Keep the refresh cookie `SameSite=None; Secure`, path-scoped to `/api/v2/auth/refresh`.
- Keep the `X-CSRF-Token` double-submit check.
- `CORS_ORIGINS` must include: `https://app.yapperhq.com`, `https://yapperhq.com`, `tauri://localhost`, `http://tauri.localhost`, `capacitor://localhost`.
- Update the frontend's `VITE_API_URL` to the tunnel host and redeploy Pages.

---

## 7. Backups — non-negotiable

Script: [`scripts/backup-postgres.sh`](../scripts/backup-postgres.sh). Run it on a schedule (Coolify scheduled task or host cron).

**Why encrypted:** the dump holds Signal identity/prekeys, Argon2 password hashes, emails, and COPPA child DOBs. R2's server-side encryption isn't enough — Cloudflare would hold the key. The script pipes `pg_dump | zstd | age` so R2 only ever receives ciphertext.

**Setup:**
1. Generate a keypair: `age-keygen -o backup-key.txt`. Put the **public** key in `AGE_RECIPIENT`. Store the **private** key OFF-box in a password manager — it is the recovery key, and losing it makes every backup permanently unreadable.
2. Create a **separate R2 bucket** for backups with its **own scoped API token** — never reuse the media token. A compromise of one must not expose the other.
3. Set an **R2 lifecycle rule** for remote retention (e.g. `daily/` expire after 7 days, `weekly/` after 8 weeks). The script deliberately doesn't script remote deletion — the storage layer already does expiry.
4. Env: `BACKUP_DATABASE_URL`, `AGE_RECIPIENT`, `R2_ACCOUNT_ID`, `R2_BACKUP_BUCKET`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`.

### Restore

```bash
age -d -i /path/to/backup-key.txt yapper-<timestamp>.sql.zst.age \
  | zstd -d \
  | psql "$TARGET_DATABASE_URL"
```

### Restore drill — do this before real users exist

A backup you have never restored is not a backup, and here a failed restore means users permanently lose message history and keys.

1. Create a scratch database.
2. Restore the newest R2 backup into it using the command above.
3. Point a backend instance at the scratch DB and confirm `/health` returns `{"db":true}` and a user can log in.
4. Drop the scratch DB.

Repeat after any major Postgres version change.

---

## 8. Post-migration cleanup

- **Remove the Neon keepalive.** `backend/src/db.rs` (~lines 24–36) runs a 240 s `SELECT 1` purely to stop Neon's 5-minute auto-suspend. Self-hosted Postgres never suspends — it's dead code once migrated.
- **Abandon the Fly deploy jobs.** `.github/workflows/ci.yml` (~lines 209–263) has commented-out `deploy-backend`/`deploy-frontend`. Backend deploys now come from Coolify; delete the backend job rather than un-commenting it. The Cloudflare Pages frontend deploy stays.
- Update `docs/deployment.md` to describe Coolify instead of Fly.io.
