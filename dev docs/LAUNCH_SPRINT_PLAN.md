# YAPPER — Launch Sprint Plan (S17–S19)

> Generated: 2026-07-16
> Source: audit of docs vs. actual source (backend + frontend verification agents + /impeccable UI audit)
> Scope: everything genuinely remaining between "code-complete" and "public launch"
> Framing: code is ~95% complete; launch *execution* is ~40%. Most remaining work is external/operational, not feature-building.

Effort key: **S** = <2h · **M** = half day · **L** = 1–2 days. Owner: BE / FE / FS (DevOps).

---

## S17 — Pre-Launch Hardening (code fixes)

Small, high-value diffs. All root-cause fixes at shared choke points (one edit covers all callers), not per-caller patches.

| # | Task | Owner | Effort | Acceptance |
|---|------|-------|--------|------------|
| 1 | **Fix silent COPPA audit-log discard.** `let _ = INSERT parental_action_audit` at `parental/mod.rs:569` (feeds 4 approve/decline sites). Add `if let Err(e) => tracing::error!`. | BE | S | Failed insert logs at `error!` with parent/child/action ids; approve/decline still succeeds. |
| 2 | **Fix silent parent-notification discard.** `let _ = INSERT parent_notifications` in `notify_parent()` `users/mod.rs:2491`. Same treatment. | BE | S | Failed insert logged, not swallowed. |
| 3 | **Wire push for channel messages.** In `fanout_to_channel_members` (`hub.rs:2036-2074`) switch loop to `try_send_to_user`, collect offline member ids, `tokio::spawn notify_user_offline_devices(uid,"channel",…)`. One edit covers human + bot channel sends. | BE | S | Channel message to a disconnected recipient generates a `type:channel` push; DM push regression test still green. |
| 4 | **Tokenize parent/ theming (light-theme blocker).** `routes/parent/children/setup/+page.svelte` (~39 hex) + `routes/parent/+layout.svelte` (18 hex). Replace text/bg/border/toggle colors with `var(--color-*)` from `app.css`; keep the intentional gradient backgrounds (mark `/* ponytail: brand gradient */`). → `/impeccable colorize routes/parent/` | FE | M | All non-decorative colors use tokens; light-theme toggle shows readable text + contrasted controls; no dark-theme regression. |
| 5 | **Hide/disable Apple sign-in stub.** `DiscordImport.svelte:41` currently toasts "coming soon" on click. Disable button + tooltip, or hide. | FE | S | No confusing dead CTA; no unhandled rejection. |
| 6 | **Confirm canvas migration 000029 applied.** `20260321000029_canvas_expansion.sql` exists; admin-role RBAC already wired (`canvas/service.rs:60-87`). Verify it's in the target env's applied set. | FS | S | Migration present in deploy env before S18 flips deploy on. No code change. |

**Deliberately NOT in scope (ponytail — leave them):** the 12 other `let _ =` sites are fire-and-forget WS pushes / already-logged / `write!`-to-String that cannot fail. Touching them is scope creep. Transfer-ownership and bot-SDK "coming soon" strings are intentional post-MVP placeholders (informational, not broken CTAs) — leave.

---

## S18 — Launch Verification (prove it works)

The 45 Playwright specs exist but have **never been run against staging**. This is the biggest unknown; everything downstream assumes they pass.

| # | Task | Owner | Effort | Acceptance |
|---|------|-------|--------|------------|
| 1 | **Seed staging test accounts + env.** Playwright needs `BASE_URL`, `VITE_API_URL`, `E2E_EMAIL`/`E2E_PASSWORD` (+ optional `_2` pair). `global-setup.ts` does a real login → no auth bypass, so accounts must exist, no 2FA, clean session state. | FS | M | 2+ seeded staging accounts log in via API; env documented. |
| 2 | **Run all 45 specs against staging, fix failures.** | FE | L | Full suite green against staging. |
| 3 | **Frontend deploy stays on Cloudflare Pages** — confirm `CLOUDFLARE_API_TOKEN` secret + auto-deploy on `main` still wired. (Backend deploy moves to Coolify — see S18b; the paused Fly jobs in `ci.yml:209-263` are now abandoned, not re-enabled.) | FS | S | Push to `main` redeploys the Pages frontend. |

> Backend hosting is migrating to self-hosted Coolify (S18b) instead of unpausing Fly.io. The Fly deploy jobs stay commented out and become dead code to remove during doc cleanup.

---

## S18b — Coolify Homelab Migration

Move the two stateful components Neon's free tier constrains onto self-hosted Coolify (homelab: 20c / 128GB / 2TB NVMe — hugely over-provisioned, so prod + staging run on one box). **Leave frontend + marketing on Cloudflare Pages and media on R2** — those are free CDN/storage with nothing to gain from moving. Decided 2026-07-16; "for now" hosting, revisit when scale demands.

> **Operator detail:** [`COOLIFY_RUNBOOK.md`](COOLIFY_RUNBOOK.md) — deploy steps, the full 39-var env/secrets checklist, Cloudflare Tunnel setup, cutover config, and the backup/restore procedure.
> **Backup script:** [`scripts/backup-postgres.sh`](../scripts/backup-postgres.sh) (encrypted `pg_dump | zstd | age` → R2).

| # | Task | Owner | Effort | Acceptance |
|---|------|-------|--------|------------|
| 1 | **Deploy backend to Coolify** from the existing `Dockerfile` (git-push/webhook deploy). Set JWT keys, R2, Resend, etc. as Coolify secrets. | FS | M | Backend builds + runs on Coolify; `/health` returns `{"db":true}`. |
| 2 | **Provision Coolify Postgres**, run `sqlx migrate run`, point backend `DATABASE_URL` at it. Kills the Neon 0.5GB cap + cold starts. | FS | S | All 38 migrations applied; app reads/writes live DB. |
| 3 | **Expose via Cloudflare Tunnel (`cloudflared`)** — no port-forwarding, hides residential IP, TLS at edge, passes WebSockets (the hub). Point `api.yapperhq.com` at the tunnel. | FS | M | `api.yapperhq.com` reachable over WSS + HTTPS via tunnel; no open inbound ports. |
| 4 | **Encrypted Postgres backups → R2** (non-negotiable — DB holds Signal keys, Argon2 hashes, child DOBs). Scheduled `pg_dump \| zstd \| age -r <key>` → **separate R2 bucket** with its **own scoped token**. Local NVMe dump for fast restore. `age` private key stored **off-box** (password manager). Retention: daily→7d, weekly→4–8wk. | FS | M | Encrypted dump lands in R2 on schedule; app's media token can't read the backup bucket. |
| 5 | **Run a restore drill.** A backup never restored isn't a backup — a failed restore = permanent key/history loss. | FS | S | Fresh Postgres restored from an R2 `.age` dump; app boots against it. |
| 6 | **Config-only cutover:** update CORS allowlist + `FRONTEND_URL` + the frontend's API base URL to the tunnel host. Stay cross-subdomain (app on Pages, api on homelab) → keep `SameSite=None` + CSRF unchanged. | FS | S | Frontend (Pages) talks to backend (homelab); auth cookies + CSRF work. |
| 7 | **Remove the Neon keepalive** (`db.rs:24-36`, 240s `SELECT 1`) — dead once Postgres is self-hosted and never auto-suspends. | BE | S | Keepalive task deleted; no idle-suspend regression. |

> **Not moving (ponytail):** frontend app + marketing (Cloudflare Pages — free CDN + DDoS; marketing's wishlist Worker is D1/KV-coupled) and media (R2 — free 10GB, no egress). Media → MinIO on Coolify is a pure env-var swap available later when consolidation is worth it; not now.
>
> **Accepted caveat:** single homelab = single point of failure (power, home internet, no redundancy). Fine for beta; exit is cheap because frontend + media stay on portable free tiers.

---

## S19 — Platform Builds & Go-Live (external, needs Mac/Linux)

| # | Task | Owner | Effort | Acceptance |
|---|------|-------|--------|------------|
| 1 | **Generate Tauri signing keys** (`cargo tauri signer generate`) → GitHub Secrets (`TAURI_SIGNING_PRIVATE_KEY` + password). | FS | S | Signed auto-updates work. |
| 2 | **macOS DMG build** — `targets:"all"` + `icon.icns` present; run on macOS agent. | FS | S | DMG builds + runs. |
| 3 | **Linux AppImage build** — run on Linux agent. | FS | S | AppImage builds + runs. |
| 4 | **GitHub Release** with Windows NSIS (done) + macOS DMG + Linux AppImage. | FS | S | Release published with 3 desktop installers. |
| 5 | **Wishlist launch email blast** via Resend Worker. | FS | S | Sent to subscribers. |

Apple FamilyControls entitlement (1–4 week lead) moves to V0.2 alongside the mobile work — it only gates real iOS screen time, which ships with native mobile.

### iOS / Android — DEFERRED to V0.2 (decided 2026-07-16)

**Launch scope is desktop (Tauri) + web PWA. Native mobile is post-launch.**

`frontend/ios` and `frontend/android` do not exist — no Capacitor platform is set up (despite older docs claiming stubs). The web PWA already covers mobile browsers, so native apps are a distinct follow-on project, not a launch dependency. Standing them up (Capacitor iOS init + Android init, ~L each, plus Apple Dev $99/yr, Play $25, and signing chains) is tracked in the V0.2 backlog below.

Launch/marketing comms: **"Desktop + Web now, mobile apps coming."**

---

## Backlog — Debt (not launch-blocking)

| Task | Owner | Effort | Notes |
|------|-------|--------|-------|
| **Refactor functions >60 lines** (67 total; NASA Rule 4). Start with `gdpr_export` at 505 lines — an 8× outlier, splits cleanly by data domain. Then `delete_account` (221), `get_profile` (168), `restore_backup_v2`/`load_service_account` (167). | BE | L | Review/maintainability only; no behavior change, keep module tests green. |
| **Native mobile (V0.2)** — Capacitor iOS + Android init, Apple FamilyControls entitlement (1–4wk lead — apply when this is scheduled), Apple Dev $99/yr + Play $25, signing chains, store submissions. | FE+FS | L+L | Distinct project; web PWA covers mobile browsers meanwhile. |
| **Mentions push** — no backend mention parser exists; channel bodies are E2E ciphertext, so mention detection must happen client-side pre-encrypt and pass explicit `mentioned_user_ids`. Protocol change, not a one-liner. | BE+FE | L+ | Needs scoping decision before sprinting. |
| **Tokenize emoji/ + profile/ colors** — minor; mostly decorative brand accents. → `/impeccable colorize lib/components/emoji/` | FE | M | Low light-theme risk; consistency pass. |
| **Real Apple sign-in, transfer-ownership, bot SDK** — the three "coming soon" features. | BE+FE | — | Post-MVP product work. |
| **Update stale docs** — HANDOVER/SPRINT_PLAN test counts (217→274 BE), "all fns <60 lines" is false, remove iOS/Android-stub claims. | — | S | Doc hygiene. |

---

## Critical path

```
S17 (code fixes, ~2 days one dev)
      │
      ▼
S18b (Coolify: backend + Postgres + tunnel + encrypted backups)  ── homelab box
      │   (staging lands here too — same box)
      ▼
S18 (seed staging → run E2E)  ← biggest unknown lives here
      │
      ▼
S19 (Mac/Linux builds + release + wishlist blast) ── needs a Mac/Linux machine
      │
      ▼
   LAUNCH (desktop + web)
      │
      └─▶ V0.2: Capacitor iOS/Android, mentions push, 60-line refactors
```

S18b comes before S18 because staging (where E2E runs) is now a second Coolify project on the same box — stand up the platform first, then verify against it. Fastest realistic path to a desktop+web public launch: **S17 + S18b + S18 ≈ 1 week of focused work**, then S19 gated only by access to a Mac/Linux build machine and the E2E suite going green.
