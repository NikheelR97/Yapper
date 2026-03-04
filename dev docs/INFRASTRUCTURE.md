# INFRASTRUCTURE & HOSTING PLAN

**Last updated:** 2026-03-04
**Status:** Living document — update when providers or pricing change
**Current tier:** Tier 0 (Free MVP)

---

## 1. Overview

Yapper's infrastructure follows a tiered model designed to keep costs at $0 through MVP, then scale
paid spend incrementally as user growth demands it. This document defines four tiers: **Tier 0**
(free, current MVP state), **Tier 1** ($5–$10/month, early growth ~1K–3K users), **Tier 2**
(~$30/month, up to 10K users), and **Tier 3** (~$250/month, up to 100K users). Each tier documents
which services to use, when to upgrade, and the exact code changes required.

> **Reference:** See `HANDOVER.md` Section 10 (Deployment) for day-to-day operational runbooks,
> secrets management, and CI/CD pipeline details.

---

## 2. Tier 0 — Free MVP (Current)

### 2a. Service Stack

| Component | Provider | Free Limit | Notes |
|-----------|----------|------------|-------|
| Backend API + WS Hub | Fly.io (3 shared VMs) | 3 always-on 256MB VMs | jnb (Johannesburg) region |
| Database | Neon (serverless PostgreSQL) | 0.5GB storage | Fills at ~500 active users with message history |
| Frontend (app) | Cloudflare Pages | Unlimited bandwidth | app.yapperhq.com — auto-deploy from GitHub |
| Marketing site | Cloudflare Pages | Unlimited bandwidth | yapperhq.com — separate Pages project |
| Media storage | Cloudflare R2 | 10GB + free egress | S3-compatible API |
| Wishlist API | Cloudflare Workers + D1 + KV | 100K requests/day | yapperhq.com/api/* |
| Transactional email | Resend | 3K emails/month | Password reset, email verify |
| Push notifications | Firebase FCM | No limit | Android + Web push |
| Error monitoring | Sentry | 5K errors/month | |

### 2b. Unavoidable One-Time Costs

| Item | Cost | When |
|------|------|------|
| Domain (yapperhq.com) | ~$10/year | Already paid |
| Apple Developer Program | $99/year | At iOS App Store submission |
| Google Play Developer | $25 one-time | At Google Play submission |

### 2c. Architecture Diagram

```
[Users]
  │
  ├─ Web/PWA ──────────────→ Cloudflare Pages (app.yapperhq.com)
  ├─ Desktop (Tauri) ──────→ Cloudflare Pages (app.yapperhq.com)
  ├─ Mobile (Capacitor) ───→ Cloudflare Pages (app.yapperhq.com)
  │
  ├─ API / WebSocket ──────→ Fly.io (1 VM, jnb) [yapper-api.fly.dev]
  │                              └── Neon PostgreSQL (0.5GB)
  │
  ├─ Media (E2EE) ─────────→ Cloudflare R2 (presigned URLs, 10GB free)
  ├─ Marketing ────────────→ Cloudflare Pages (yapperhq.com)
  ├─ Wishlist ─────────────→ Cloudflare Workers + D1 + KV
  └─ Push ─────────────────→ Firebase FCM
```

---

## 3. Tier 1 — Early Growth: $5–$10/month (~1K–3K Users)

### 3a. When to Upgrade

Upgrade to Tier 1 when **any** of the following is hit:
- Neon free tier approaches 0.5GB storage (typical at ~500 active users with full message history)
- R2 media storage approaches 10GB free limit
- Monthly media uploads consistently exceed 5GB

Everything else stays on free tier. **Only media storage changes at this tier.**

### 3b. Primary Change — Media: Cloudflare R2 → AWS S3 + Glacier Instant Retrieval

**Why replace R2?**

R2 has no cold storage tier. Every GB costs $0.015/month regardless of how old or how rarely
accessed. Once users accumulate months of old media (photos, video clips, audio yaps), most of
it is never re-accessed after 30 days. Glacier Instant Retrieval (IR) costs $0.004/GB/month with
**millisecond access latency** — no restore delays, no UX change, 83% cheaper for aged media.

The migration effort is minimal: R2 already uses the S3-compatible API. Swapping to real S3 is
an endpoint URL + credentials change only.

#### Storage Tier Comparison

| Tier | Provider | Cost/GB/month | Latency | Egress |
|------|----------|--------------|---------|--------|
| Active media (0–30 days) | S3 Standard | $0.023 | ms | 100GB/mo free |
| Aged media (30+ days) | S3 Glacier IR | $0.004 | ms | 100GB/mo free |
| All media (no cold tier) | Cloudflare R2 | $0.015 | ms | Free always |

#### R2 vs S3 + Glacier IR at Scale

| Storage | R2 (no cold tier) | S3 + Glacier IR |
|---------|------------------|----------------|
| 50GB hot | $0.75 | $1.15 |
| 150GB aged | $2.25 | $0.60 |
| 300GB archived | $4.50 | $1.20 |
| **500GB total** | **$7.50/mo** | **$2.95/mo** |

S3 + Glacier IR wins at scale. R2 is only cheaper for small volumes with no old media.

#### Azure Blob Storage Alternative

| | Azure Blob | S3 + Glacier IR |
|--|-----------|----------------|
| Hot tier | $0.018/GB ✅ cheaper | $0.023/GB |
| Cool tier (30–90 days) | $0.010/GB | — |
| Cold tier (90+ days) | $0.0036/GB | $0.004/GB (Glacier IR) |
| Early deletion fee | ✅ Yes (30/90 day minimum) | ❌ None |
| S3 API compatibility | ❌ Requires `@azure/storage-blob` SDK | ✅ Same API as R2 |
| Verdict | ⚠️ Cheaper hot, pricier aged | ✅ **Recommended** |

**Decision:** S3 + Glacier IR wins for Yapper. Migration from R2 is an endpoint swap. Azure would
require a new SDK and has early deletion fees that add billing complexity.

#### S3 Lifecycle Policy

Apply this once on the S3 bucket. AWS automatically moves objects to Glacier IR after 30 days:

```json
{
  "Rules": [
    {
      "ID": "MoveToGlacierIRAfter30Days",
      "Status": "Enabled",
      "Filter": { "Prefix": "" },
      "Transitions": [
        {
          "Days": 30,
          "StorageClass": "GLACIER_IR"
        }
      ]
    }
  ]
}
```

### 3c. Cost Summary

| Component | Provider | Cost |
|-----------|----------|------|
| Backend | Fly.io (free 3 VMs) | $0 |
| Database | Neon Free | $0 |
| Frontend | Cloudflare Pages | $0 |
| Marketing | Cloudflare Pages | $0 |
| Media — hot (~50GB active) | S3 Standard | ~$1.15 |
| Media — aged (~150GB) | S3 Glacier IR | ~$0.60 |
| CDN / egress | AWS (100GB/mo free) | $0 |
| Wishlist | Cloudflare Workers | $0 |
| **Total** | | **~$2–7/month** |

---

## 4. Tier 2 — 10K Users (~$30/month)

### 4a. When to Upgrade

Upgrade to Tier 2 when **any** of the following is hit:
- Concurrent WebSocket connections regularly exceed **1,200** (single 256MB VM cap)
- Neon storage approaches 0.5GB
- Backend OOM kills visible in `fly logs`

### 4b. What Breaks at 10K

| Component | Breaks At | Symptom |
|-----------|-----------|---------|
| Single Fly.io VM (256MB) | ~1,200 concurrent WS | OOM kills, dropped connections |
| Neon Free (0.5GB) | ~500 active users | Disk full, write failures |
| In-memory WS hub (single node) | 2nd VM | User A on VM1 can't deliver to User B on VM2 |
| No cache | Heavy read traffic | DB CPU spikes on profile/server loads |

### 4c. Backend — Multi-VM + pg LISTEN/NOTIFY Fan-Out

The current in-memory hub (`Arc<RwLock<HashMap>>`) is single-node. When a second VM is added,
each VM only knows about its own WebSocket connections. Cross-VM delivery requires a message bus.

**Solution:** PostgreSQL LISTEN/NOTIFY (already planned in architecture). Each VM subscribes to
relevant channels; messages published by any VM are received by all VMs and delivered to local
connections.

```
[User A on VM1] → sends message
  └─ VM1 → NOTIFY pg_channel('ch:{channel_id}', payload)
       └─ VM2 (has User B) LISTEN → deliver to User B ✅
       └─ VM3 (no recipients) → ignore ✅
```

**Fly.io scaling:**
```bash
fly scale count 3          # Adds 2 more VMs instantly
fly scale show             # Verify
```

**Code changes:** `backend/src/hub.rs`
- Each VM calls `LISTEN channel_messages` on startup
- `broadcast_to_channel()` emits `pg_notify('channel_messages', json_payload)`
- Each VM's listener delivers to locally-connected users

**Cost:** ~$4–6/mo for 2 additional shared-1x VMs (256MB each)

### 4d. Database — Neon Launch

Upgrade Neon plan in the Neon dashboard (no migration needed — same connection strings):

| Plan | Storage | Compute | Cost |
|------|---------|---------|------|
| Neon Free | 0.5GB | 0.25 vCPU (shared) | $0 |
| **Neon Launch** | **10GB** | **Autoscaling** | **$19/mo** |
| Neon Scale | 50GB | Autoscaling + read replicas | $69/mo |

**Schema change — message table partitioning:**

Partition the `messages` table by month to keep query performance constant as messages grow
into tens of millions of rows:

```sql
-- Migration: convert messages to partitioned table
-- Run as migration 000016 or later
ALTER TABLE messages RENAME TO messages_old;

CREATE TABLE messages (
  id UUID NOT NULL,
  channel_id UUID,
  conversation_id UUID,
  sender_id UUID NOT NULL,
  ciphertext BYTEA NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Create initial partition (backfill from messages_old separately)
CREATE TABLE messages_2026_03 PARTITION OF messages
  FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');

-- Add a default partition for future months (or create monthly via cron)
CREATE TABLE messages_default PARTITION OF messages DEFAULT;
```

### 4e. In-Process Cache — moka (Rust crate)

Add `moka` to `backend/Cargo.toml` for per-VM in-process caching. Zero infra cost, zero ops.

```toml
moka = { version = "0.12", features = ["future"] }
```

Add to `AppState` in `backend/src/main.rs`:

```rust
use moka::future::Cache;

pub struct AppState {
    // ... existing fields ...
    pub user_cache: Cache<Uuid, UserProfile>,    // 5-min TTL
    pub member_cache: Cache<Uuid, Vec<Member>>,  // 2-min TTL
}

// Init:
let user_cache = Cache::builder()
    .time_to_live(Duration::from_secs(300))
    .max_capacity(10_000)
    .build();
```

**What to cache:**

| Data | TTL | Key |
|------|-----|-----|
| User profiles | 5 min | `user_id` |
| Server member lists | 2 min | `server_id` |
| Channel permissions | 1 min | `(channel_id, user_id)` |
| Prekey bundle availability | 30 sec | `user_id` |

### 4f. Media — S3 + Glacier + Optional CloudFront

CloudFront CDN is optional at 10K users but recommended when media GETs/sec spikes. At ~1TB total
media with 85% CloudFront cache hit rate:

| Line item | Cost |
|-----------|------|
| S3 Standard (100GB active) | $2.30 |
| S3 Glacier IR (900GB aged) | $3.60 |
| CloudFront (optional) | ~$1–2 |
| **Total media** | **~$6–8/mo** |

### 4g. Code Changes Required (10K)

| Priority | Change | File |
|----------|--------|------|
| 1 | pg LISTEN/NOTIFY cross-VM fan-out | `backend/src/hub.rs` |
| 2 | Monthly message table partitioning | New SQL migration |
| 3 | moka in-process cache in AppState | `backend/src/main.rs` + handlers |
| 4 | Fly.io scale count: `fly scale count 3` | Config (no code) |

### 4h. Cost Summary

| Component | Provider | Cost |
|-----------|----------|------|
| Backend (3 VMs) | Fly.io shared-1x | ~$4–6 |
| Database | Neon Launch | $19 |
| Media (S3 + Glacier, ~1TB) | AWS | ~$6 |
| CloudFront (optional) | AWS | ~$0–2 |
| Redis | None at this tier | $0 |
| **Total** | | **~$29–33/month** |

---

## 5. Tier 3 — 100K Users (~$250/month)

### 5a. When to Upgrade

Upgrade to Tier 3 when **any** of the following is hit:
- pg LISTEN/NOTIFY saturating (~50K+ WS messages/sec through PG)
- Cross-VM cache inconsistency causing stale data bugs (moka inconsistency across 15 VMs)
- Neon Launch approaching 10GB storage
- Global users reporting high latency (single-region jnb limitation)
- Media egress costs spiking without CloudFront

### 5b. What Breaks at 100K

| Component | Breaks At | Symptom |
|-----------|-----------|---------|
| pg LISTEN/NOTIFY | ~50K msg/sec | PG CPU at 100%, notify lag |
| Neon Launch (10GB) | ~10K active users with history | Disk full |
| moka (per-VM cache) | 15+ VMs | 15 separate stale caches |
| Single region (jnb) | Global users | 200ms+ latency for non-African users |
| S3 egress without CDN | ~5TB/month served | $500+/month egress bills |
| Prekey supply | 100K concurrent X3DH sessions | Prekey exhaustion |

### 5c. Message Bus — Redis Pub/Sub (replaces pg LISTEN/NOTIFY)

**Provider:** Upstash Redis Pro ($10/mo — managed, auto-scales, no ops)

```
[User sends msg] → VM7
  └─ VM7 → PUBLISH channel:abc {payload}
       ├─ VM1 SUBSCRIBE (has recipient A) → deliver ✅
       ├─ VM3 SUBSCRIBE (has recipient B) → deliver ✅
       └─ All other VMs → receive + discard (no local connections for this channel) ✅
```

**Rust implementation** (`backend/src/hub.rs`):
```rust
// Replace pg LISTEN/NOTIFY publish with:
redis_client.publish(format!("channel:{}", channel_id), payload).await?;

// Each VM subscribes on startup:
let mut pubsub = redis_client.get_async_pubsub().await?;
pubsub.subscribe("channel:*").await?;
```

**Alternative:** Self-hosted NATS on Fly.io (~$4/mo, 1M msg/sec) — more work, more throughput,
no managed autoscaling.

### 5d. Database — Neon Scale or AWS RDS

| Option | Cost | Trade-off |
|--------|------|-----------|
| **Neon Scale** | $69/mo | Zero migration, autoscaling, read replicas included ✅ |
| AWS RDS db.t4g.medium | ~$30/mo | Cheaper, manual ops, separate from current Neon setup |
| Supabase Pro | $25/mo | 8GB cap — too small |

**Recommendation:** Neon Scale — no migration, read replicas enable read/write split.

**Read/write split pattern:**
- All `INSERT`/`UPDATE`/`DELETE` → primary pool (`state.db.pool()`)
- All `SELECT` for profiles/explore/search → read replica pool (`state.db.read_pool()`)

**Additional indexes:**
```sql
CREATE INDEX CONCURRENTLY idx_messages_channel_time
  ON messages (channel_id, created_at DESC);

CREATE INDEX CONCURRENTLY idx_messages_user_conv
  ON messages (sender_id, conversation_id, created_at DESC);
```

### 5e. Backend — Multi-Region Deployment

```bash
# Add regions
fly regions add ams iad          # Amsterdam + Virginia

# Scale across regions
fly scale count 5 --region jnb
fly scale count 5 --region ams
fly scale count 5 --region iad
```

Redis pub/sub handles cross-region fan-out automatically — VMs in all regions subscribe to the
same Upstash Redis instance. No additional cross-region coordination needed.

**Sticky WebSocket sessions:** Fly.io `fly-replay` header routes reconnects to the same VM.
Already works — no code change.

**Cost:** 15 VMs × ~$2/mo = ~$30/mo

### 5f. Shared Cache — Upstash Redis (same instance as pub/sub)

Replace per-VM moka caches with shared Redis. All 15 VMs read/write the same cache.

```rust
// Replace moka Cache<K, V> with Redis GET/SET + TTL
async fn get_user_profile(redis: &RedisClient, user_id: Uuid) -> Result<UserProfile> {
    let key = format!("profile:{}", user_id);
    if let Some(cached) = redis.get::<String>(&key).await? {
        return Ok(serde_json::from_str(&cached)?);
    }
    let profile = db_fetch_profile(user_id).await?;
    redis.set_ex(&key, serde_json::to_string(&profile)?, 300).await?;
    Ok(profile)
}
```

**Cache entries:**

| Data | TTL | Key pattern |
|------|-----|-------------|
| User profiles | 5 min | `profile:{user_id}` |
| Server member lists | 2 min | `members:{server_id}` |
| Channel permissions | 1 min | `perms:{channel_id}:{user_id}` |
| Prekey availability | 30 sec | `opk_count:{user_id}` |
| Online presence | Real-time via hub | `presence:{user_id}` |

**Cost:** Included in Upstash Redis Pro $10/mo (covers both pub/sub + cache)

### 5g. Media — CDN Mandatory

Without CloudFront at 5TB/month egress:
- S3 egress: $0.09/GB × 5,000GB = **$450/month** ❌

With CloudFront (85% cache hit rate):
- Only 750GB reaches S3 = **$67/month** for egress ✅

**CloudFront setup:**
- Create CloudFront distribution → S3 bucket origin
- Use CloudFront-signed URLs instead of S3 presigned URLs in `backend/src/media/`
- Set Cache-Control headers on upload: `max-age=31536000` (media is immutable after upload)

**Client-side compression (before E2EE encryption):**

Add to frontend before calling upload endpoint:

```typescript
// Images: resize to max 2048px, convert to WebP
async function compressImage(file: File): Promise<Blob> {
    const bitmap = await createImageBitmap(file, { resizeWidth: 2048, resizeHeight: 2048,
        resizeQuality: 'medium' });
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    canvas.getContext('2d')!.drawImage(bitmap, 0, 0);
    return canvas.convertToBlob({ type: 'image/webp', quality: 0.82 });
}

// Videos: cap at 720p via MediaRecorder constraints (enforce during capture)
```

**User storage quotas** (new DB column + enforcement):
```sql
ALTER TABLE users ADD COLUMN media_bytes_used BIGINT NOT NULL DEFAULT 0;
-- Free: 1GB limit, GoPro: 50GB limit
```

**Lambda@Edge** for automatic thumbnail generation on upload (optional, reduces bandwidth for
media previews in chat lists).

### 5h. E2EE Challenges at Scale

#### Prekey Exhaustion
With 100K users simultaneously initiating X3DH sessions, one-time prekeys (OPKs) deplete fast.

**Fix:** Background job in `backend/src/keys/` checks OPK count per user every 15 minutes.
When count < 20, sends a push notification via FCM:
```rust
// FCM payload prompts client to upload fresh OPK batch
{ "type": "replenish_prekeys", "current_count": 5, "requested": 100 }
```
Client handles this silently in the background, uploads 100 new OPKs.

#### Sender Key Distribution Storms
A 500-member channel rotating Sender Keys = 500 ECIES key distributions triggered simultaneously.

**Fix:** Lazy distribution in `backend/src/servers/service.rs`:
- On Sender Key rotation: distribute only to **currently online** members immediately
- For offline members: store in `pending_key_distributions` table
- Deliver queued distributions on user's next WebSocket connect

#### Key Backup Thundering Herd
100K users hitting `GET /api/v1/keys/backup` simultaneously after a deploy.

**Fix:** Cache backup status flag in Redis (`backup_exists:{user_id}`, 10-min TTL).
Only hit DB on actual backup/restore operation.

### 5i. Operational Requirements (Cannot Skip at 100K)

| Need | Solution | Cost |
|------|----------|------|
| Error monitoring | Sentry Business | ~$26/mo |
| Log aggregation | Fly.io built-in + Axiom free | $0 |
| Rate limiting | Redis-backed (uses existing Upstash) | $0 |
| DB backups | Neon Scale (daily backups included) | $0 |
| DDoS protection | Cloudflare proxy (already configured) | $0 |
| GDPR export/delete | Backend endpoints (code-only) | $0 |
| Abuse detection | Message frequency heuristics (code-only) | $0 |
| Storage quotas | DB column + quota enforcement (code-only) | $0 |

### 5j. Code Changes Required (100K)

| Priority | Change | Files |
|----------|--------|-------|
| 1 | Redis pub/sub fan-out replacing pg LISTEN/NOTIFY | `backend/src/hub.rs` |
| 2 | Redis shared cache replacing moka | `backend/src/main.rs` + all handlers |
| 3 | Read replica connection pool routing | `backend/src/db.rs` |
| 4 | CloudFront presigned URL generation | `backend/src/media/` |
| 5 | Prekey replenishment background job | `backend/src/keys/` (new file) |
| 6 | Lazy Sender Key distribution | `backend/src/servers/service.rs` |
| 7 | Client-side image/video compression | `frontend/src/lib/media/compress.ts` (new) |
| 8 | User storage quota tracking | New migration + `backend/src/media/` |
| 9 | Multi-region Fly.io config | `fly.toml` |

### 5k. Cost Summary

| Component | Provider | Cost |
|-----------|----------|------|
| Backend (15 VMs, 3 regions) | Fly.io shared-1x | ~$30 |
| Database | Neon Scale | $69 |
| Message bus + shared cache | Upstash Redis Pro | $10 |
| Media hot (~2TB S3 Standard) | AWS S3 | $46 |
| Media aged (~15TB Glacier IR) | AWS Glacier IR | $60 |
| CDN | AWS CloudFront | ~$15 |
| Error monitoring | Sentry Business | ~$26 |
| **Total** | | **~$256/month** |

---

## 6. Master Scaling Summary

| Component | Tier 0 (Free, MVP) | Tier 1 (~$5, 1K–3K users) | Tier 2 (~$30, 10K users) | Tier 3 (~$250, 100K users) |
|-----------|-------------------|--------------------------|--------------------------|---------------------------|
| Backend VMs | 1x Fly.io free | 1x Fly.io free | 3x Fly.io shared | 15x Fly.io multi-region |
| Database | Neon Free (0.5GB) | Neon Free | Neon Launch (10GB) $19 | Neon Scale (50GB) $69 |
| Message bus | pg LISTEN/NOTIFY | pg LISTEN/NOTIFY | pg LISTEN/NOTIFY | Redis pub/sub (Upstash) |
| Cache | None | None | moka (in-process, per-VM) | Redis shared (Upstash) |
| Media | R2 (10GB free) | S3 + Glacier IR | S3 + Glacier IR | S3 + Glacier IR |
| CDN | None | None | Optional (CloudFront) | Mandatory (CloudFront) |
| Frontend | Cloudflare Pages | Cloudflare Pages | Cloudflare Pages | Cloudflare Pages |
| Regions | jnb | jnb | jnb | jnb + ams + iad |
| E2EE | Unchanged | Unchanged | Unchanged | + prekey replenishment |
| **Monthly cost** | **$0** | **~$2–7** | **~$29–33** | **~$250–260** |

---

## 7. Migration Runbooks

### Tier 0 → Tier 1: R2 to S3 + Glacier IR

1. **Create AWS S3 bucket** in `us-east-1` (or closest region to jnb — `af-south-1` for Africa)
2. **Create IAM user** with least-privilege policy:
   ```json
   {
     "Effect": "Allow",
     "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject", "s3:GetObjectAttributes"],
     "Resource": "arn:aws:s3:::yapper-media/*"
   }
   ```
3. **Apply lifecycle policy** (JSON in Section 3b above) via AWS Console or CLI:
   ```bash
   aws s3api put-bucket-lifecycle-configuration \
     --bucket yapper-media \
     --lifecycle-configuration file://lifecycle.json
   ```
4. **Stage Fly.io secrets:**
   ```bash
   fly secrets set \
     AWS_ACCESS_KEY_ID=... \
     AWS_SECRET_ACCESS_KEY=... \
     AWS_BUCKET=yapper-media \
     AWS_REGION=af-south-1
   ```
5. **Update `backend/src/media/`** — swap R2 custom endpoint URL to S3:
   ```rust
   // Before (R2):
   .endpoint_url("https://<account_id>.r2.cloudflarestorage.com")
   // After (S3):
   // Remove endpoint_url — S3 uses default AWS endpoint
   ```
6. **Migrate existing R2 objects** (if any):
   ```bash
   rclone copy r2:yapper-media s3:yapper-media --transfers 32
   ```
7. **Validate** presigned URL generation returns correct S3 URLs in staging

---

### Tier 1 → Tier 2: Multi-VM + Neon Launch + moka Cache

1. **Upgrade Neon plan** in [console.neon.tech](https://console.neon.tech) → Project → Settings → Billing → Launch ($19/mo)
2. **Run message partitioning migration** (new SQL migration file in `backend/migrations/`)
3. **Deploy hub.rs with pg LISTEN/NOTIFY fan-out** (cross-VM message delivery)
4. **Add moka to Cargo.toml** + wire into AppState
5. **Scale Fly.io:**
   ```bash
   fly scale count 3
   fly status    # Verify 3 machines running
   ```
6. **Monitor** `fly logs` for cross-VM delivery working correctly

---

### Tier 2 → Tier 3: Redis + Multi-Region + CloudFront

1. **Create Upstash Redis Pro** at [upstash.com](https://upstash.com) → copy `REDIS_URL`
2. **Stage secret:**
   ```bash
   fly secrets set REDIS_URL=rediss://...
   ```
3. **Swap hub.rs fan-out** from pg LISTEN/NOTIFY to Redis pub/sub
4. **Swap AppState cache** from moka to Redis GET/SET
5. **Add read replica pool** in `backend/src/db.rs`
6. **Add Fly.io regions:**
   ```bash
   fly regions add ams iad
   fly scale count 5 --region jnb
   fly scale count 5 --region ams
   fly scale count 5 --region iad
   ```
7. **Create CloudFront distribution** in AWS Console:
   - Origin: S3 bucket
   - Viewer protocol: HTTPS only
   - Cache policy: Managed-CachingOptimized
8. **Update `backend/src/media/`** to generate CloudFront-signed URLs
9. **Set Cache-Control on upload:** `max-age=31536000, immutable`
10. **Monitor** Redis pub/sub latency and CloudFront cache hit rate

---

## 8. Decision Log

| Decision | Rationale |
|----------|-----------|
| **R2 → S3 at Tier 1** | Cold tier economics: Glacier IR at $0.004/GB vs R2 flat $0.015/GB. At 500GB+ aged media, savings exceed S3 hot tier premium. S3 API = endpoint-only change from R2. |
| **Azure Blob rejected** | Glacier IR ($0.004) is cheaper than Azure Cool ($0.01) for 30–90 day data. Azure early deletion fees add billing risk. New SDK required (no S3 compatibility). |
| **pg LISTEN/NOTIFY at Tier 2** | Avoids Redis dependency through 10K users. PG LISTEN/NOTIFY handles ~50K notify/sec — sufficient headroom. Keeps infra simple and $0. |
| **Upstash Redis at Tier 3** | Managed auto-scaling removes ops burden. Single instance serves both pub/sub and cache. Alternative: self-hosted NATS (higher throughput, more work). |
| **Neon Scale over AWS RDS** | Zero migration path (same Neon project, just upgrade plan). Autoscaling handles spiky load. Read replicas included. RDS saves ~$40/mo but adds manual ops. |
| **Fly.io over AWS ECS/GCP** | Fly.io already in use (Tier 0), multi-region is `fly regions add`. No Kubernetes, no load balancer config, no VPC setup. Shared VMs are affordable at all tiers. |
| **jnb first, then ams + iad** | Primary users in Southern Africa (Johannesburg). Add Europe + Americas only when user distribution justifies it (Tier 3+). |
| **No Redis at Tier 2** | moka in-process cache is sufficient for 3 VMs. Cache inconsistency across 3 nodes is acceptable — stale data window is 1–5 minutes. Redis added only when VMs scale to 10+. |

---

*Cross-reference: `HANDOVER.md` Section 10 — Deployment runbooks, secrets management, CI/CD*
*Cross-reference: `SPRINT_PLAN.md` — Phase timeline and feature delivery context*
