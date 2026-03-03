# YAPPER — Comprehensive Code Review

**Reviewer:** Senior Software Engineer
**Date:** 2026-03-03
**Scope:** Full codebase (Rust backend, SvelteKit frontend, Signal crypto layer)
**Standards Reference:** `dev docs/HANDOVER.md` Section 3 (NASA/JPL Rules) + Section 4 (Security Standards)

---

## SECURITY & VULNERABILITY ASSESSMENT

### HIGH Severity

**S-H1. CSRF Exemption Too Broad** — `backend/src/csrf.rs`
```rust
// CURRENT (line ~33)
if path.contains("/auth/") {
    return Ok(next.run(req).await);
}
```
`path.contains("/auth/")` exempts ALL routes containing `/auth/`, including state-changing endpoints like `POST /auth/refresh` and `DELETE /auth/logout`. An attacker can craft a cross-site form POST to logout a user without a CSRF token.

**Fix:** Whitelist specific exempt paths:
```rust
const CSRF_EXEMPT: &[&str] = &[
    "/auth/oauth/discord",
    "/auth/oauth/google",
    "/auth/oauth/discord/callback",
    "/auth/oauth/google/callback",
];
if CSRF_EXEMPT.iter().any(|p| path.starts_with(p)) {
    return Ok(next.run(req).await);
}
```

---

**S-H2. TOCTOU Race in Parental Server-Join Interception** — `backend/src/servers/service.rs`
```rust
// Check parental flag (line ~384)
let user_row = sqlx::query("SELECT parental_controls_enabled FROM users WHERE id = $1")
    .bind(user_id).fetch_one(pool).await?;
// ... time passes ...
// Insert membership (line ~395)
sqlx::query("INSERT INTO server_memberships ...")
```
Between SELECT and INSERT, a concurrent request could toggle `parental_controls_enabled`, bypassing the approval workflow.

**Fix:** Wrap in a transaction with `FOR UPDATE`:
```rust
let mut tx = state.db.pool().begin().await?;
let user_row = sqlx::query(
    "SELECT parental_controls_enabled FROM users WHERE id = $1 FOR UPDATE"
).bind(user_id).fetch_one(&mut *tx).await?;
// ... check + insert inside same tx ...
tx.commit().await?;
```

---

**S-H3. OAuth State Map Unbounded Growth** — `backend/src/auth/oauth.rs`
The `OAuthStateStore` (`DashMap<String, Instant>`) is only garbage-collected when new OAuth redirects are initiated. An attacker can generate thousands of state tokens via automated requests without completing the flow, exhausting memory.

**Fix:** Add a max-size cap and periodic cleanup:
```rust
if store.len() > 10_000 {
    gc_oauth_states(&store);
    if store.len() > 10_000 {
        return Err(AppError::RateLimited);
    }
}
```

---

**S-H4. Unbounded DM Fan-Out Query** — `backend/src/hub.rs`
```rust
// DM peers query has NO LIMIT
let dm_rows = sqlx::query(
    "SELECT user_id FROM dm_participants WHERE conversation_id IN (
       SELECT conversation_id FROM dm_participants WHERE user_id = $1
     ) AND user_id != $1"
).fetch_all(pool).await.unwrap_or_default();
```
A power user in 1000+ DM conversations triggers a query returning thousands of rows and an O(n) fan-out loop per presence change. The server_memberships query has `LIMIT $2` but DMs do not.

**Fix:**
```rust
"... AND user_id != $1 LIMIT 500"
```

---

**S-H5. Missing Rate Limits on Sensitive Endpoints**
Per HANDOVER.md §4 ("5 failed logins → 15-min lockout"), rate limiting is required on auth endpoints. The following endpoints lack per-user rate limits:

| Endpoint | Risk |
|----------|------|
| `POST /api/v1/parental/children` | Child account spam |
| `PATCH /parental/friend-requests/{id}` | Approval enumeration |
| `POST /api/v1/canvas/polls` | Poll flood in a channel |
| `PATCH /api/v1/users/me` | Profile update spam |
| OAuth callbacks | Automated account creation |

**Fix:** Apply `governor` per-user limiter to all state-changing endpoints, not just login.

---

**S-H6. OPK Deleted After Decryption, Not Before** — `frontend/src/lib/signal/index.ts`
```typescript
// Line ~277: decrypt first, delete OPK second
const plaintext = await decryptDm(ciphertext, ...);
await deleteOPK(opkId);  // If decrypt throws, OPK is reused
```
If `decryptDm()` throws, the OPK is never consumed. An attacker can repeatedly send malformed first-messages to reuse the same OPK, violating forward secrecy guarantees.

**Fix:** Delete OPK before attempting decryption:
```typescript
await deleteOPK(opkId);
try {
    return await decryptDm(ciphertext, ...);
} catch (e) {
    // OPK already consumed — this is the correct behavior
    throw e;
}
```

---

### MEDIUM Severity

**S-M1. Search Query Unbounded Length** — `backend/src/explore/mod.rs`
```rust
if q.is_empty() { return Err(...); }
// No max-length check — a 1MB query string causes CPU spike via pg_trgm similarity()
```
**Fix:** `if q.len() > 255 { return Err(AppError::BadRequest("Search query too long".into())); }`

---

**S-M2. Poll Option Text Unbounded** — `backend/src/canvas/mod.rs`
Poll option count is validated (2–10), but individual option text has no length limit. 10 options × 10MB each = 100MB request body.

**Fix:**
```rust
for opt in &body.options {
    if opt.len() > 500 { return Err(AppError::BadRequest("Option too long".into())); }
}
```

---

**S-M3. `debug_assert!` on User Input** — `backend/src/servers/service.rs`
```rust
debug_assert!(!name.is_empty(), "name must be non-empty");
```
Per HANDOVER.md §5 ("Never trust client input — validate at the boundary, assert at the core"), `debug_assert!` is stripped in release builds. User input must use runtime validation.

**Fix:** Replace with `if name.is_empty() { return Err(AppError::BadRequest(...)); }`

---

**S-M4. Hype Moments Query Unbounded** — `backend/src/users/mod.rs`
```rust
"SELECT id, message_id, type, pinned_at FROM hype_moments WHERE user_id = $1 ORDER BY pinned_at DESC"
// No LIMIT — if 9-limit is bypassed via race, memory allocation is unbounded
```
**Fix:** Add `LIMIT 100`.

---

**S-M5. Silent `let _ =` on Fallible DB Operations** — `backend/src/hub.rs`, `backend/src/parental/mod.rs`
```rust
let _ = sqlx::query("UPDATE messages SET delivered = TRUE WHERE id = ANY($1)")
    .execute(pool).await;
```
Per HANDOVER.md §7 ("The return value of non-void functions must be checked"), ignoring DB errors without logging violates Rule 7.

**Fix:**
```rust
if let Err(e) = sqlx::query("UPDATE messages SET delivered = TRUE ...")
    .execute(pool).await {
    tracing::warn!("Failed to mark delivered: {e}");
}
```

---

**S-M6. `unwrap_or_default()` Masks Schema Errors** — `backend/src/parental/mod.rs`
```rust
let account_type: String = caller_row.try_get("account_type").unwrap_or_default();
```
An empty string default silently passes all `account_type != "parent"` checks if the column doesn't exist, masking a schema mismatch.

**Fix:** `let account_type: String = caller_row.try_get("account_type")?;`

---

**S-M7. IndexedDB Double-Initialization Race** — `frontend/src/lib/signal/keystore.ts`
```typescript
async function getDB() {
    if (_db) return _db;
    _db = await openDB(...);  // Two concurrent callers both enter here
}
```
**Fix:**
```typescript
let _dbPromise: Promise<IDBPDatabase> | null = null;
async function getDB() {
    if (_db) return _db;
    if (!_dbPromise) { _dbPromise = openDB(...).then(db => { _db = db; return db; }); }
    return _dbPromise;
}
```

---

**S-M8. Backup DB Version Mismatch** — `frontend/src/lib/signal/backup.ts` vs `frontend/src/lib/signal/keystore.ts`
```typescript
// backup.ts: DB_VERSION = 1
// keystore.ts: DB_VERSION = 2 (added sender key stores in v2)
```
Backup export uses v1 schema and will not include sender key stores. A user who restores from backup loses all group encryption state.

**Fix:** Align `DB_VERSION` in backup.ts to 2 and export all v2 stores.

---

### LOW Severity

**S-L1.** `getrandom::getrandom().expect()` in `auth/service.rs` — panics in prod if entropy exhausted. Return `AppResult` instead.

**S-L2.** OAuth access token in redirect URL query string (`oauth.rs` line ~423) — JWTs are ~1KB, nearing URL length limits. Consider session-based exchange.

**S-L3.** `generate_invite_code()` uses `.unwrap_or_default()` on `SystemTime::now()` — makes nanos=0, reducing code entropy.

---

## USER EXPERIENCE DEGRADATION

### HIGH Severity

**U-H1. Silent Auth Failure — No User Feedback** — `frontend/src/routes/(app)/+layout.svelte`
```typescript
catch {
    await goto("/login");  // No toast, no error message
    return;
}
```
User is silently redirected to login with no indication of why. If their session expired, they see a blank login page.

**Fix:**
```typescript
catch {
    toast.error("Session expired. Please sign in again.");
    await goto("/login");
}
```

---

**U-H2. Silent Signal Key Setup Failure** — `frontend/src/routes/(app)/+layout.svelte`
```typescript
catch (e) {
    console.warn("[Signal] Key setup failed:", e);
}
```
If key setup fails, the user enters the app unable to encrypt or decrypt any messages, with no warning. Every send/receive will fail individually with cryptic errors.

**Fix:**
```typescript
catch (e) {
    console.error("[Signal] Key setup failed:", e);
    toast.error("Encryption setup failed. Messages may not work. Try refreshing.");
}
```

---

**U-H3. Decryption Failures Are Invisible** — `frontend/src/lib/stores/conversations.ts`
```typescript
catch {
    return { ...m, text: null, decryptError: true };  // No UI indicator
}
```
Messages that fail to decrypt appear as empty gaps in the conversation. Users see missing messages with no explanation or ability to retry.

**Fix:** Render a visible error card: `"This message couldn't be decrypted. The sender may need to re-send it."`

---

**U-H4. WebSocket Disconnect Invisible to User** — `frontend/src/lib/stores/ws.ts`
The `wsStore` tracks `{ connected: boolean }` but no component reads this to show a disconnect banner. The user can type and send messages that silently fail.

**Fix:** Add a connection-status banner in the app layout:
```svelte
{#if !$wsStore.connected}
  <div class="connection-banner">Reconnecting...</div>
{/if}
```

---

### MEDIUM Severity

**U-M1. No Loading State During Message Send** — `frontend/src/routes/(app)/dm/[conversationId]/+page.svelte`
The `sending` flag disables the button but provides no visual indication (spinner, opacity change). Fast networks mask this, but on slow connections the UI appears frozen.

---

**U-M2. Empty States Lack Error Context** — Multiple stores
`fetchConversations()` and `fetchServers()` failures resolve with `.catch(() => {})`. On network errors, the user sees empty conversation/server lists with no indication of failure vs. genuinely empty state.

---

**U-M3. DOB Validation Allows Invalid Dates** — `frontend/src/routes/parent/children/setup/+page.svelte`
```typescript
new Date(year, month - 1, day)  // Accepts Feb 30 → silently becomes Mar 2
```
**Fix:** Validate the constructed date matches the inputs:
```typescript
const dob = new Date(year, month - 1, day);
if (dob.getDate() !== day) { toast.error("Invalid date"); return; }
```

---

**U-M4. Hardcoded Version String** — `frontend/src/routes/(app)/settings/+page.svelte`
`v2.4.0` hardcoded in the nav sidebar. Will become stale immediately.

---

### LOW Severity

**U-L1.** Presence indicator (`dm/+page.svelte`) lacks `aria-label` — screen readers get no semantic status info.

**U-L2.** Child setup wizard DOB inputs lack grouped `aria-describedby` for screen readers.

**U-L3.** `ready` flag in app layout is set once and never reset on WebSocket reconnection.

---

## COMPANY STANDARDS COMPLIANCE (HANDOVER.md)

### HIGH Severity

**C-H1. Rule 4 Violation: Functions Exceeding 60 Lines**

| Function | File | Lines | Limit |
|----------|------|-------|-------|
| `get_profile()` | users/mod.rs | **115** | 60 |
| `gdpr_export()` | users/mod.rs | **99** | 60 |
| `create_or_get_conversation()` | messages/mod.rs | **81** | 60 |
| `update_profile()` | users/mod.rs | **69** | 60 |

**Fix:** Extract DB queries and response-building into helper functions per the `handlers.rs → service.rs` pattern documented in §3 Rule 4.

---

**C-H2. Rule 5 Violation: Missing Precondition Assertions in Service Functions**
HANDOVER.md §3 Rule 5: "Every `service.rs` function must validate its preconditions before proceeding."

| Function | File | Missing |
|----------|------|---------|
| `do_join()` | servers/service.rs | No `debug_assert!` on `server_id` non-nil |
| `create_child()` | parental/mod.rs | No age-range assertion on DOB |
| `create_poll()` | canvas/mod.rs | No assertion that `channel_id` belongs to caller's server |

---

**C-H3. Rule 7 Violation: Unchecked Return Values**
Per §3 Rule 7: "Never use `let _ = fallible_call()` without a comment explaining why."

Found 4 instances of `let _ =` on DB operations without justification comments:
- `hub.rs` line ~429 (mark delivered)
- `parental/mod.rs` line ~312 (parent notification)
- `hub.rs` line ~340 (presence broadcast failure)
- `canvas/mod.rs` (poll vote duplicate check)

---

### MEDIUM Severity

**C-M1. Rule 1 Violation: Deep Nesting**
`get_child_overview()` in `parental/mod.rs` has 5 levels of nesting (fetch → map → try_get → unwrap_or → conditional). Should use guard clauses.

---

**C-M2. Handlers Contain Business Logic**
HANDOVER.md §3 Rule 4: "Keep `handlers.rs` thin. Business logic lives in `service.rs`."

Several modules have all logic in `mod.rs` with no `service.rs` split:
- `parental/mod.rs` — handlers + service combined (~400 lines)
- `users/mod.rs` — handlers + service combined (~900 lines)
- `keys/mod.rs` — handlers + service combined (~300 lines)
- `canvas/mod.rs` — combined (~350 lines)
- `explore/mod.rs` — combined (~250 lines)

**Fix:** Split each into `handlers.rs` + `service.rs` per the documented module pattern in §5.

---

**C-M3. Rule 3 Violation: Missing Capacity Bounds**
HANDOVER.md §3 Rule 3: "Set explicit capacity limits on all unbounded collections."

The hub's `DashMap` connections per user is checked (`MAX_CONNECTIONS_PER_USER: 5`), but:
- `away_timers: DashMap` has no size cap
- `typing_timers: DashMap` has no size cap
- `away_users: DashMap` has no size cap

---

### LOW Severity

**C-L1.** Missing `#[serde(deny_unknown_fields)]` on several request DTOs (§4 "Input Validation").

**C-L2.** Several backend modules lack any unit tests (parental, canvas, explore, users).

**C-L3.** No `EXPLAIN ANALYZE` documentation comments on complex queries.

---

## PRIORITIZED ACTION PLAN

### Immediate (Pre-Production Blockers)

| # | Finding | Fix effort |
|---|---------|------------|
| 1 | S-H1: CSRF exemption too broad | 15 min |
| 2 | S-H5: Add rate limits to parental/OAuth/profile endpoints | 2 hrs |
| 3 | U-H1–H4: Add toast feedback for auth/signal/WS failures | 1 hr |
| 4 | S-H6: Delete OPK before decryption | 10 min |
| 5 | S-M8: Fix backup DB version mismatch | 15 min |

### Before Beta

| # | Finding | Fix effort |
|---|---------|------------|
| 6 | S-H2: TOCTOU race in parental join | 30 min |
| 7 | S-H4: Add LIMIT to DM fan-out query | 5 min |
| 8 | S-M1–M2: Add length bounds to search + poll options | 15 min |
| 9 | S-M3: Replace `debug_assert!` with runtime validation | 30 min |
| 10 | S-M5: Replace `let _ =` with logged error handling | 30 min |
| 11 | C-H1: Refactor 4 functions exceeding 60 lines | 2 hrs |
| 12 | U-M3: Fix DOB validation | 10 min |

### Long-Term (Technical Debt)

| # | Finding | Fix effort |
|---|---------|------------|
| 13 | C-M2: Split 5 combined modules into handlers/service | 4 hrs |
| 14 | C-M3: Add capacity bounds to DashMap collections | 1 hr |
| 15 | C-H2–H3: Add precondition assertions + return value checks | 2 hrs |
| 16 | S-H3: Add OAuth state map size cap + background GC | 1 hr |
| 17 | U-L1–L2: Accessibility improvements | 1 hr |

---

**Total findings: 40** (Security: 17, UX: 11, Standards: 12)
**Critical blockers for production: 5** — all fixable in a single session.
