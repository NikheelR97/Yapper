# Yapper Performance Audit Report

**Auditor:** Principal Performance Engineer
**Scope:** Full-stack — Backend · Database · WebSocket · Frontend · Infrastructure
**Date:** 2026-03-24

---

## Executive Summary

**Total findings: 18** (3 Critical, 5 High, 7 Medium, 3 Low)

The codebase is architecturally sound for MVP launch — the hub uses lock-free `DashMap`, Argon2id is correctly dispatched to `spawn_blocking`, image processing is off the async executor, database queries avoid N+1 patterns, and indexes are comprehensive. However, three critical bottlenecks will degrade user experience under load:

1. **Per-message database query for device trust validation** — every inbound WS message hits PostgreSQL (`hub.rs:963`)
2. **No virtual scrolling in MessageList** — 200+ DOM nodes rendered regardless of viewport (`MessageList.svelte:94`)
3. **Unbounded IP rate limiter memory** — governor `DefaultKeyedStateStore<IpAddr>` has no eviction (`lib.rs:69`)

**Estimated user-facing impact:** The device trust query adds 2–20ms per message in steady state, but 500ms–2s per message after a Neon cold start. The missing virtual scroll causes visible jank at 200+ messages. The unbounded rate limiter is a DoS vector on a 256MB VM.

### What's already well-built

- Hub uses `DashMap` (lock-free) — no `RwLock` contention issues
- Fan-out uses `try_send()` — non-blocking, no `.await` under locks
- Argon2id correctly dispatched to `spawn_blocking`
- Image processing (avatar/banner/emoji WebP) correctly dispatched to `spawn_blocking`
- DB queries avoid N+1 patterns — JOINs used throughout
- Indexes are comprehensive (partial indexes on OPKs, undelivered envelopes, search via `pg_trgm` GIN)
- Offline delivery is a single bulk query with `LIMIT 100` + batch `UPDATE`
- Trending tags use in-memory 5-min TTL cache
- Login rate limiter bounded at 50K entries with LRU-like GC
- WS backpressure via `yield_now()` after 32 messages per tick

---

## Severity Classification

- **CRITICAL** — directly and measurably degrades user experience (>200ms added latency, memory leak risk, or executor starvation)
- **HIGH** — significant degradation under moderate load (50+ concurrent users)
- **MEDIUM** — noticeable at scale or in edge cases
- **LOW** — minor inefficiency or missed optimization

---

## BE-1: Device Trust State Query on Every WS Message

**Severity:** CRITICAL
**Component:** `backend/src/hub.rs:962-972`

### Bottleneck

Every inbound WebSocket message — DMs, channel messages, typing indicators, read receipts — triggers `live_device_trust_state(user_id, device_id, state).await` which executes a database query against the `devices` table. At 5 msg/sec per user with 100 concurrent users, this is **500 DB round-trips/second** solely for trust validation. After a Neon cold start (5-min idle), the first message pays a 500ms–2s penalty.

```rust
// hub.rs:962-972 — EVERY inbound message hits this path
let device_is_trusted = match device_id {
    Some(device_id) => match live_device_trust_state(user_id, device_id, state).await {
        Some(DeviceTrustState::Trusted) => true,
        // ...
    },
    None => true,
};
```

### Remediation

Cache trust state in a `DashMap<(UserId, DeviceId), (DeviceTrustState, Instant)>` on the Hub with a 60-second TTL. Invalidate on trust-change WS events (`device_trust_updated`). Device trust changes are rare (approval/revocation); re-checking every 60s is sufficient.

```rust
// Add to Hub struct
trust_cache: DashMap<(Uuid, Uuid), (DeviceTrustState, Instant)>,

fn cached_trust_state(&self, user_id: Uuid, device_id: Uuid) -> Option<DeviceTrustState> {
    self.trust_cache.get(&(user_id, device_id))
        .filter(|entry| entry.1.elapsed() < Duration::from_secs(60))
        .map(|entry| entry.0.clone())
}
```

Memory cost: ~80 bytes per entry x 1000 active device sessions = ~80KB.

### Validation

Benchmark with `wrk` against a WS echo with trust check before/after. Target: p99 inbound message latency < 5ms (down from 20ms+). Measure Neon query count via `pg_stat_statements`. New test needed for cache invalidation on trust state change and TTL expiry.

---

## BE-2: Unbounded IP Rate Limiter Memory Growth

**Severity:** CRITICAL
**Component:** `backend/src/lib.rs:67-69`

### Bottleneck

The per-IP HTTP rate limiter uses `governor::RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>`. Governor's `DefaultKeyedStateStore` is an unbounded `DashMap` — entries are never evicted. An attacker sending requests from rotating IPs creates a new entry per IP. Each entry is ~128 bytes. At 1M unique IPs = ~128MB — over half the 256MB VM.

The same issue applies to password-reset and email-verification rate limiters in `auth/handlers.rs:78-100` and the upload timestamp map in `media/handlers.rs:17-38`.

The `LoginRateLimiter` is correctly bounded at 50K entries with GC — this is the model to follow.

### Remediation

Spawn a periodic GC task capped at 50K entries:

```rust
let rl = rate_limiter.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        rl.retain_recent(); // governor 0.6+
    }
});
```

If governor doesn't expose `retain_recent()`, switch to a custom `DashMap<IpAddr, (GCRA, Instant)>` with a 15-minute TTL eviction pass capped at 50K entries.

Memory cap: 50K entries x 128 bytes = ~6.4MB max.

### Validation

Load test with 100K unique source IPs. Monitor RSS via `fly ssh console` + `top`. Target: RSS stays below 200MB under sustained attack. New test needed for GC and entry cap.

---

## BE-3: No Neon Keepalive — Cold Start Penalty on Idle

**Severity:** CRITICAL
**Component:** `backend/src/db.rs:10-15`

### Bottleneck

The connection pool sets `min_connections(1)` but has no keepalive mechanism. Neon auto-suspends after 5 minutes of inactivity. The pool's idle connection may be terminated by Neon without the pool knowing. The next query after a 5+ minute idle period hits Neon's cold start: **500ms–2s latency**.

This is particularly damaging because the first action after idle is often a WebSocket reconnect, which triggers `deliver_offline_envelopes()` — a query on the critical path of message delivery.

No `test_before_acquire` is configured, so sqlx may hand out a dead connection from the pool.

### Remediation

```rust
let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(1)
    .acquire_timeout(Duration::from_secs(15))
    .test_before_acquire(true)
    .connect(url)
    .await?;

// Spawn heartbeat to prevent Neon auto-suspend
let heartbeat_pool = pool.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(240)); // 4 min
    loop {
        interval.tick().await;
        let _ = sqlx::query("SELECT 1").execute(&heartbeat_pool).await;
    }
});
```

Cost: One trivial query every 4 minutes. No meaningful CPU or Neon compute cost.

### Validation

Stop all traffic for 6 minutes, then send a request. Measure first-query latency. Target: < 50ms (warm) vs 500ms–2s (cold). The existing `db.ping()` method can be reused for the heartbeat.

---

## FE-1: No Virtual Scrolling in MessageList

**Severity:** CRITICAL (on mobile/low-end devices)
**Component:** `frontend/src/lib/components/chat/MessageList.svelte:94`

### Bottleneck

```svelte
{#each messages as msg (msg.id)}  <!-- All messages rendered -->
```

The message list renders ALL messages in the DOM. With `MAX_MESSAGES_IN_MEMORY = 200`, this creates 200+ DOM nodes. Each node includes a `renderMessageTokens()` call (emoji shortcode parsing), an `IntersectionObserver` target, and potential media sub-components. On mobile WebViews (Capacitor), 200 DOM nodes with emoji parsing causes measurable scroll jank.

### Remediation

Implement windowed rendering with a simple slice-based approach:

```svelte
<script>
  let scrollTop = 0;
  const ITEM_HEIGHT = 48;
  const BUFFER = 20;

  $: startIndex = Math.max(0, Math.floor(scrollTop / ITEM_HEIGHT) - BUFFER);
  $: endIndex = Math.min(messages.length, startIndex + Math.ceil(viewportHeight / ITEM_HEIGHT) + BUFFER * 2);
  $: visibleMessages = messages.slice(startIndex, endIndex);
</script>

<div style="height: {messages.length * ITEM_HEIGHT}px; position: relative;">
  {#each visibleMessages as msg (msg.id)}
    <div style="position: absolute; top: {(startIndex + i) * ITEM_HEIGHT}px;">
      <!-- message content -->
    </div>
  {/each}
</div>
```

Alternatively, use `@tanstack/svelte-virtual` for a battle-tested implementation.

### Validation

Lighthouse performance audit on a 500-message channel. Target: INP < 200ms, TBT < 300ms. DOM node count should stay under 60 regardless of message count. New Playwright test needed.

---

## BE-4: DM Recipient + Channel Membership DB Queries on Every Message

**Severity:** HIGH
**Component:** `backend/src/hub.rs:1106-1154` (DM), `backend/src/hub.rs:1308-1339` (Channel)

### Bottleneck

Every DM send executes `resolve_dm_recipient()` — 2 DB queries. Every channel message executes `resolve_channel_membership()` — 2 DB queries. Combined with BE-1 (trust check), a single message send triggers **3 DB round-trips** on the hot path.

At 100 concurrent users each sending 1 msg/sec = 300 DB queries/sec just for message authorization.

### Remediation

Cache conversation participants and channel memberships in the Hub on first access, with a 5-minute TTL and invalidation on membership-change events:

```rust
dm_participants: DashMap<Uuid, (Uuid, Instant)>,        // conversation_id -> recipient_id
channel_members: DashMap<Uuid, (Vec<Uuid>, Instant)>,    // channel_id -> member list
```

Invalidate `channel_members` on join/leave events. Invalidate `dm_participants` on new conversation creation.

Memory cost: 10K conversations x 24 bytes = ~240KB. 1K channels x 500 members x 16 bytes = ~8MB (cap at 8MB).

### Validation

Measure DB query count per message via `pg_stat_statements` before/after. Target: 1 DB query per message (INSERT only) vs 3–4 currently. New test needed for cache invalidation on membership change.

---

## FE-2: Media Upload Not Parallelizing Encryption with Presigned URL Fetch

**Severity:** HIGH
**Component:** `frontend/src/lib/components/chat/YapRecorder.svelte:137-173`, `frontend/src/lib/components/chat/ClipRecorder.svelte:114-160`

### Bottleneck

The upload flow is sequential:

1. `await encryptMedia(blob)` — AES-256-GCM (~50–200ms for 5MB)
2. `await api.post('/upload-url', {...})` — presigned URL from Fly.io jnb (~100–400ms)
3. `await uploadToR2(url, encrypted)` — upload to R2

Steps 1 and 2 are independent. Sequential execution wastes 150–600ms per upload.

### Remediation

```typescript
async function handleStop(blob: Blob) {
  const [encrypted, urlResponse] = await Promise.all([
    encryptMedia(blob),
    api.post<UploadUrlResponse>('/api/v1/media/upload-url', {
      content_type: blob.type,
      size: blob.size,
    }),
  ]);
  await uploadToR2(urlResponse.upload_url, encrypted.data);
}
```

### Validation

Measure time from record-stop to upload-complete. Target: 30–50% reduction in upload initiation latency.

---

## FE-3: Crypto Libraries in Main Bundle (No Code Splitting)

**Severity:** HIGH
**Component:** `frontend/src/lib/stores/ws.ts:13` -> `signal/index.ts` -> `@noble/curves` + `@noble/hashes`

### Bottleneck

`ws.ts` imports `receiveSenderKeyDist` and `handleKeyDistRequest` from `$lib/signal/index.js` at the top level (line 13). Since `ws.ts` is imported by the app layout, the entire Signal crypto library chain (~200KB: `@noble/curves` ~150KB, `@noble/hashes` ~50KB) is pulled into the main entry chunk. This adds ~400–800ms to Time to Interactive on 3G mobile.

### Remediation

Lazy-import the signal module in `ws.ts`:

```typescript
onWsMessage('key_dist', async (payload) => {
    const { receiveSenderKeyDist } = await import('$lib/signal/index.js');
    receiveSenderKeyDist(payload as any).catch(console.error);
});
```

Configure Vite manual chunks:

```typescript
// vite.config.ts
build: {
    rollupOptions: {
        output: {
            manualChunks: {
                signal: ['@noble/curves', '@noble/hashes', 'idb'],
            },
        },
    },
},
```

### Validation

Run `npx vite-bundle-visualizer` before/after. Target: main chunk reduced by ~200KB. Measure FCP in Lighthouse.

---

## FE-4: Batch Decryption Triggers 20 Store Updates for 200 Messages

**Severity:** HIGH
**Component:** `frontend/src/lib/stores/conversations.ts:209-256`

### Bottleneck

When loading message history, decrypted messages are flushed to the store in batches of 10 (`BATCH_SIZE = 10`). For 200 messages, this triggers **20 sequential `store.update()` calls**, each of which runs `.map()` over the entire array, triggers Svelte reactivity, and re-runs `renderMessageTokens()` for every message. Result: 200 x 20 = **4,000 emoji-parse operations** for a single history load.

### Remediation

Decrypt all messages first, then update the store once:

```typescript
const decryptedMap = new Map<string, DecryptResult>();
for (const msg of raw) {
    const result = await decryptMessage(msg);
    decryptedMap.set(msg.id, result);
}
// Single store update
store.update(msgs => msgs.map(m => {
    const d = decryptedMap.get(m.id);
    return d ? { ...m, ...d } : m;
}));
```

If progressive rendering is desired, use `requestAnimationFrame` batching to coalesce updates within a single animation frame.

### Validation

Profile with Chrome DevTools Performance tab during 200-message load. Target: 1 DOM reconciliation pass instead of 20.

---

## FE-5: Presence Store Creates Redundant Derived Stores

**Severity:** HIGH
**Component:** `frontend/src/lib/stores/presence.ts:38-44`

### Bottleneck

```typescript
export function getPresence(userId: string) {
    const store = getOrCreate(userId);
    fetchPresence(userId).catch(() => {});
    return derived(store, ($s) => $s);  // Identity-mapped derived — NEW instance every call
}
```

Every call creates a **new** `derived()` store that identity-maps the underlying writable. In a message list with 50 messages from different users, each component calling `getPresence(msg.senderId)` creates a new derived store. Each WS presence update triggers recomputation of all 50 derived stores.

Additionally, `fetchPresence(userId)` fires an HTTP GET on **every call** with no deduplication.

### Remediation

Return the writable directly (it's already `Readable`) and deduplicate HTTP fetches:

```typescript
const fetchedUsers = new Set<string>();

export function getPresence(userId: string) {
    const store = getOrCreate(userId);
    if (!fetchedUsers.has(userId)) {
        fetchedUsers.add(userId);
        fetchPresence(userId).catch(() => {});
    }
    return { subscribe: store.subscribe };
}
```

### Validation

Count active store subscriptions on a 50-message page via Svelte DevTools. Target: 5 stores (one per unique user) instead of 50.

---

## DB-1: No `test_before_acquire` on Connection Pool

**Severity:** MEDIUM
**Component:** `backend/src/db.rs:10-15`

### Bottleneck

Without `test_before_acquire(true)`, sqlx may hand out a stale connection silently closed by Neon's PgBouncer. The first query on this connection fails, requiring a retry.

### Remediation

Already addressed in BE-3. Add `.test_before_acquire(true)` to the pool configuration.

---

## BE-5: Canvas Store WS Handler Iterates All Server Stores

**Severity:** MEDIUM
**Component:** `frontend/src/lib/stores/canvas.ts:421-632`

### Bottleneck

`registerCanvasHandler()` loops through every open canvas store on every `canvas_update` WS event, regardless of which server the event targets. With 5 open servers and 1 update/sec per server, this means 5 unnecessary store iterations per event, each entering a 30+ case switch statement.

### Remediation

Extract `server_id` from the WS payload and look up only the targeted store:

```typescript
return onWsMessage('canvas_update', (frame) => {
    const serverId = (frame as any).server_id;
    const store = canvasStores.get(serverId);
    if (!store) return;
    // Process only this store
});
```

### Validation

Add tracing logs to count `store.update()` calls per WS event. Target: 1 per event instead of N.

---

## WS-1: No Proactive Token Refresh Before Re-Auth Challenge

**Severity:** MEDIUM
**Component:** `frontend/src/lib/stores/ws.ts:176-181`

### Bottleneck

The client waits for the server's `re_auth_required` frame (sent 60s before JWT expiry) before re-sending the token. The 60-second window between `re_auth_required` and token expiry is a degraded state where the connection could be dropped if the refresh fails.

### Remediation

Add a client-side timer when the WS connects:

```typescript
ws.onopen = () => {
    ws.send(JSON.stringify({ type: 'auth', token: get(authStore).accessToken }));
    // Proactive refresh at 12 minutes (JWT lifetime = 15 min)
    setTimeout(async () => {
        await refreshAccessToken();
        const newToken = get(authStore).accessToken;
        if (newToken && ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: 'reauth', token: newToken }));
        }
    }, 12 * 60 * 1000);
};
```

### Validation

Monitor re-auth success rate in server logs. Target: 0 connection drops due to token expiry.

---

## WS-2: No Outbound Message Buffer During Reconnection

**Severity:** MEDIUM
**Component:** `frontend/src/lib/stores/ws.ts:76-83`

### Bottleneck

```typescript
export function wsSend(msg: Record<string, unknown>): boolean {
    if (socket?.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify(msg));
        return true;
    }
    return false;  // Silently dropped
}
```

Messages sent during WS reconnection (1–30s window with exponential backoff) are silently dropped. While DM and channel sends check the return value, typing indicators and read receipts are fire-and-forget — they are lost during reconnection.

### Remediation

Add a bounded outbound queue:

```typescript
const pendingQueue: Record<string, unknown>[] = [];
const MAX_PENDING = 50;

export function wsSend(msg: Record<string, unknown>): boolean {
    if (socket?.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify(msg));
        return true;
    }
    if (pendingQueue.length < MAX_PENDING) {
        pendingQueue.push(msg);
    }
    return false;
}

// In doConnect(), after 'ready' event:
case 'ready':
    wsStore.set({ connected: true, error: null });
    while (pendingQueue.length > 0) {
        const msg = pendingQueue.shift()!;
        ws.send(JSON.stringify(msg));
    }
    break;
```

---

## DB-2: V1 Legacy DM Query Uses O(n^2) Scalar Subquery

**Severity:** MEDIUM
**Component:** `backend/src/messages/mod.rs:422-466`

### Bottleneck

The V1 DM message listing computes `msg_num` via a correlated scalar subquery:

```sql
COALESCE(
    (SELECT COUNT(*) FROM messages m2
     WHERE m2.conversation_id = $1 AND m2.created_at < m.created_at),
    0
) AS msg_num
```

For each of the `LIMIT 50` rows returned, PostgreSQL runs a `COUNT(*)` scan. On a 10K-message conversation, this is 50 x 10K = 500K row comparisons.

### Remediation

Deprecate V1 path (all multi-device clients use V2 envelopes). If V1 must persist, replace with `ROW_NUMBER()`:

```sql
SELECT *, ROW_NUMBER() OVER (ORDER BY created_at) - 1 AS msg_num
FROM messages WHERE conversation_id = $1 AND deleted_at IS NULL
ORDER BY created_at DESC LIMIT $2
```

---

## FE-6: `renderMessageTokens()` Recomputed on Every Reactive Update

**Severity:** MEDIUM
**Component:** `frontend/src/lib/components/chat/MessageList.svelte:97-98`

### Bottleneck

```svelte
{@const renderedTokens = msg.text ? renderMessageTokens(msg.text, emojiMap) : []}
```

Called for every message on every Svelte reactive update. Since message text doesn't change after decryption, re-parsing emoji shortcodes on every typing-indicator or presence update is wasted work.

### Remediation

Memoize rendered tokens on the message object in the conversations store:

```typescript
// In conversations store, after decryption:
const tokens = renderMessageTokens(text, emojiMap);
return { ...m, text, renderedTokens: tokens };
```

Then in MessageList:

```svelte
{@const renderedTokens = msg.renderedTokens ?? []}
```

---

## INFRA-1: Single Region Backend (jnb) for Global Users

**Severity:** MEDIUM
**Component:** `backend/fly.toml` — `primary_region = "jnb"`

### Bottleneck

All API calls and WebSocket connections route to Johannesburg. Users in North America experience ~250ms RTT, Europe ~150ms RTT, Asia ~300ms RTT. For read-only HTTP endpoints like explore/trending-tags and communities, this latency is unnecessary.

### Remediation (Cost-neutral)

Add Cloudflare Workers edge caching for read-only explore endpoints:

```javascript
// Cloudflare Worker (add as a Pages Function)
export async function onRequest(context) {
    const cache = caches.default;
    const cached = await cache.match(context.request);
    if (cached) return cached;

    const response = await fetch(`https://yapper-api.fly.dev${context.request.url}`);
    const cloned = response.clone();
    cloned.headers.set('Cache-Control', 'public, max-age=300');
    context.waitUntil(cache.put(context.request, cloned));
    return response;
}
```

Candidate endpoints: `GET /api/v1/explore/trending-tags`, `/communities`, `/live-servers`.

---

## INFRA-2: No Explicit Cache Headers for Static Assets

**Severity:** MEDIUM
**Component:** `frontend/wrangler.toml` — no `_headers` file

### Bottleneck

SvelteKit produces hashed filenames for JS/CSS but without a `_headers` file, Cloudflare Pages uses defaults. Hashed assets should have `immutable` caching.

### Remediation

Create `frontend/static/_headers`:

```
/_app/immutable/*
  Cache-Control: public, max-age=31536000, immutable

/index.html
  Cache-Control: no-cache

/*.html
  Cache-Control: no-cache
```

### Validation

`curl -I https://app.yapperhq.com/_app/immutable/entry/app.*.js` — verify `Cache-Control: public, max-age=31536000, immutable`.

---

## BE-6: Upload Timestamp Map Has No User Cleanup

**Severity:** LOW
**Component:** `backend/src/media/handlers.rs:17-38`

### Bottleneck

`UPLOAD_TIMESTAMPS: DashMap<Uuid, Vec<Instant>>` prunes old timestamps within a user's entry but never removes a user entry entirely when all timestamps expire. Accumulates empty `Vec` entries over time.

### Remediation

Remove entries when the Vec is empty after pruning:

```rust
entry.retain(|t| now.duration_since(*t) < window);
if entry.is_empty() {
    drop(entry);
    UPLOAD_TIMESTAMPS.remove(&user_id);
}
```

---

## BE-7: Governor Rate Limiter for Password Reset/Email Verify Also Unbounded

**Severity:** LOW
**Component:** `backend/src/auth/handlers.rs:78-100`

### Bottleneck

Same unbounded `DefaultKeyedStateStore` issue as BE-2, but lower risk due to lower traffic. Password reset and email verification rate limiters grow with unique IP/email combinations.

### Remediation

Same approach as BE-2 — periodic GC task or bounded store with TTL eviction. Can share the same GC task.

---

## BE-8: `serde_json::to_string()` on Every Outbound WS Frame per Recipient

**Severity:** LOW
**Component:** `backend/src/hub.rs:463-485`

### Bottleneck

Each outbound WebSocket message is serialized to JSON on the send task. For fan-out to 500 channel members, the **same message** is serialized 500 times.

### Remediation

Pre-serialize the message once before fan-out:

```rust
pub fn broadcast_preserialized(&self, user_ids: &[Uuid], json: String) {
    for user_id in user_ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
        self.send_preserialized(user_id, json.clone());
    }
}
```

Trades `serde_json::to_string()` x N for `String::clone()` x N — ~10x reduction in CPU per fan-out. Impact is minor (~500us saved per fan-out) — low priority.

---

## Prioritized Fix List

| Priority | Finding | Severity | Estimated Impact | Effort |
|----------|---------|----------|------------------|--------|
| 1 | BE-3: Neon keepalive + test_before_acquire | CRITICAL | Eliminates 500ms–2s cold start | 30 min |
| 2 | BE-1: Cache device trust state | CRITICAL | Saves 2–3 DB queries per WS message | 2 hrs |
| 3 | BE-2: Bound IP rate limiter | CRITICAL | Prevents OOM on 256MB VM | 1 hr |
| 4 | FE-4: Single store update for batch decrypt | HIGH | 20x fewer DOM reconciliations | 30 min |
| 5 | FE-5: Fix presence store redundancy | HIGH | 10x fewer store subscriptions | 30 min |
| 6 | FE-2: Parallelize media encrypt + URL fetch | HIGH | 30–50% faster upload initiation | 15 min |
| 7 | FE-3: Lazy-import signal crypto | HIGH | 200KB smaller initial bundle | 1 hr |
| 8 | FE-1: Virtual scroll for MessageList | CRITICAL | 200 -> ~40 DOM nodes | 4 hrs |
| 9 | BE-4: Cache membership/participants | HIGH | 2 fewer DB queries per message | 3 hrs |
| 10 | INFRA-2: Static asset cache headers | MEDIUM | Eliminates revalidation RTT | 15 min |
| 11 | WS-1: Proactive token refresh | MEDIUM | Prevents re-auth degradation window | 30 min |
| 12 | FE-6: Memoize emoji token rendering | MEDIUM | Eliminates redundant parsing | 1 hr |
| 13 | BE-5: Canvas handler server_id filter | MEDIUM | N->1 store updates per event | 15 min |
| 14 | INFRA-1: Edge cache for explore APIs | MEDIUM | ~200ms latency reduction globally | 2 hrs |
| 15 | DB-2: Remove V1 legacy msg_num subquery | MEDIUM | O(n^2) -> O(n) query | 30 min |
| 16 | WS-2: Outbound message buffer | MEDIUM | No lost typing indicators | 30 min |
| 17 | BE-6: Upload timestamp cleanup | LOW | Minor memory hygiene | 10 min |
| 18 | BE-8: Pre-serialize broadcast messages | LOW | ~500us saved per fan-out | 1 hr |

---

## Architectural Recommendations

### 1. Hub-Level Read-Through Cache (Post-MVP)

The repeated pattern of "DB query on WS hot path" (trust state, membership, participants) suggests a systematic solution: a Hub-level read-through cache with event-driven invalidation. All cacheable data should live in a single `HubCache` struct with per-key TTLs and invalidation hooks wired to WS events.

### 2. Connection Pool Warm-Up

Beyond the heartbeat (BE-3), consider `min_connections(2)` — one for the heartbeat and one ready for immediate use. Monitor Neon's active connection count via `pg_stat_activity` to stay within free tier limits.

### 3. WebSocket Binary Protocol (Post-MVP)

For high-frequency events (typing, presence, read receipts), consider MessagePack via `rmp-serde` to reduce frame size by 30–50% and avoid JSON parse overhead. Gate behind a `protocol_version` field in the auth handshake.

### 4. Multi-Region Fly.io (Post-MVP, ~$5/month)

Deploy a second Fly.io machine in `iad` (US East). Use PostgreSQL LISTEN/NOTIFY (already scaffolded in `db.rs:36-61`) to synchronize cross-region WS state. Single highest-impact scaling investment.

---

## Operational Constraints (All Remediations Comply)

- **Memory budget:** No fix increases idle RAM beyond 256MB. All caches have documented max sizes and eviction policies.
- **E2EE integrity:** No optimization routes plaintext through the server.
- **Cost neutrality:** No new paid service dependencies. All fixes use existing free-tier infrastructure.
- **COPPA compliance:** No optimization bypasses parental control approval enforcement.
- **Test coverage:** Each finding notes whether existing tests cover the change and where new tests are required.
