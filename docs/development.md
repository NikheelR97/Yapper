# Development Guide

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.78+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 20+ | [nodejs.org](https://nodejs.org) |
| Docker | any | [docker.com](https://www.docker.com) |
| sqlx-cli | latest | `cargo install sqlx-cli --no-default-features --features postgres` |
| cargo-watch | latest | `cargo install cargo-watch` |

Optional for desktop:
- [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) (WebView2 on Windows, webkit2gtk on Linux)

---

## Initial Setup

### 1. Clone

```bash
git clone https://github.com/NikheelR97/Yapper.git
cd Yapper
```

### 2. Environment

```bash
cp .env.example backend/.env
# Edit backend/.env — at minimum set DATABASE_URL
```

### 3. JWT Keys

```bash
cd backend/secrets
openssl genrsa -out jwt_private.pem 2048
openssl rsa -in jwt_private.pem -pubout -out jwt_public.pem
cd ../..
```

These are gitignored. Never commit them.

### 4. Database

```bash
make db-up       # starts Docker postgres on port 5432
make migrate     # runs all 12 sqlx migrations
```

Verify:

```bash
docker exec -it $(docker ps -qf name=postgres) psql -U yapper -c '\dt'
```

### 5. Backend

```bash
make dev-backend   # cargo watch -x run
# → http://localhost:8080
# → http://localhost:8080/health  (should return {"status":"ok","db":true})
```

### 6. Frontend

```bash
cd frontend && npm install && cd ..
make dev-frontend  # → http://localhost:5173
```

### 7. Desktop (optional)

```bash
make dev-tauri
```

---

## Backend Development

### Adding a new endpoint

1. Find (or create) the module in `backend/src/<module>/mod.rs`
2. Write the handler using `sqlx::query()` (non-macro) + `.try_get()` — no need to re-run `cargo sqlx prepare`
3. Add the route to the module's `router()` function
4. `cargo check` before pushing

**Pattern:**

```rust
async fn my_handler(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<MyInput>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query("SELECT ... FROM ... WHERE id = $1")
        .bind(some_id)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("Not found".into()))?;

    Ok(Json(serde_json::json!({ "field": row.try_get::<String, _>("field")? })))
}
```

### Adding a migration

```bash
cd backend
sqlx migrate add my_migration_name
# edit migrations/TIMESTAMP_my_migration_name.sql
make migrate
```

If you use `sqlx::query!` macros (we generally avoid this), regenerate the cache:

```bash
make sqlx-prepare   # must use non-pooler Neon URL
```

### Error types

All handlers return `AppResult<T>` (alias for `Result<T, AppError>`).

| `AppError` variant | HTTP status |
|-------------------|-------------|
| `NotFound(msg)` | 404 |
| `BadRequest(msg)` | 400 |
| `Unauthorized` | 401 |
| `Forbidden` | 403 |
| `Conflict(msg)` | 409 |
| `TooManyRequests` | 429 |
| `Database(sqlx::Error)` | 500 |
| `Internal(anyhow::Error)` | 500 |

---

## Frontend Development

### Adding a new store

Create `frontend/src/lib/stores/my-feature.ts`:

```typescript
import { writable } from 'svelte/store';
import { api } from '$api/client.js';

interface MyState { items: Item[]; loading: boolean; }
const initial: MyState = { items: [], loading: false };
export const myStore = writable<MyState>(initial);

export async function loadItems() {
    myStore.update(s => ({ ...s, loading: true }));
    try {
        const data = await api.get<{ items: Item[] }>('/my-endpoint');
        myStore.update(s => ({ ...s, items: data.items }));
    } finally {
        myStore.update(s => ({ ...s, loading: false }));
    }
}
```

### Registering a WS handler

```typescript
import { onWsMessage } from '$stores/ws.js';
import { onMount, onDestroy } from 'svelte';

let unregister: (() => void) | null = null;

onMount(() => {
    unregister = onWsMessage('my_event_type', (frame) => {
        // handle frame
    });
});

onDestroy(() => unregister?.());
```

### Adding a new route

Create `frontend/src/routes/(app)/my-page/+page.svelte`. It will be protected by the auth guard in the `(app)` layout automatically.

---

## Code Conventions

### Rust

- `sqlx::query()` (non-macro) for all new queries — avoids `cargo sqlx prepare` on every change
- `debug_assert!` for preconditions that are validated at the call site
- `pub(crate)` for functions shared between modules; `pub` only for public API
- Error propagation with `?` — no `.unwrap()` in handler code
- No `unwrap()` / `expect()` in production paths; use `ok_or_else(|| AppError::...)`

### TypeScript / Svelte

- Always import with `.js` extension in `$lib/signal/` (Web Crypto / noble compatibility)
- `Uint8Array.prototype.slice()` before passing noble outputs to `crypto.subtle` (type narrowing)
- Stores are the single source of truth — components only read stores and call store actions

---

## Testing

### Backend unit tests

```bash
make test-backend   # cargo test
```

Tests live alongside the code in `#[cfg(test)]` modules. Auth service has tests for hashing, JWT, and token validation.

### Frontend type checking

```bash
cd frontend && npm run check   # svelte-check + TypeScript
```

### E2E (Playwright — not yet implemented)

```bash
make test-e2e
```

For live-account Playwright runs, generate local auth-state artifacts before running the suite:

```bash
cd frontend
npm run test:setup-auth
```

This writes `.gitignore`d files under `frontend/tests/auth-state/`:

- `user-a.json` / `user-a.data.json`
- `user-b.json` / `user-b.data.json` when `E2E_EMAIL_2` and `E2E_PASSWORD_2` are set

Parallel workers must clone fresh browser contexts from those files instead of reusing a single shared authenticated page/session.

### Self-hosted runner disk maintenance

Self-hosted GitHub Actions runners keep browser binaries and work directories on disk between jobs. Run the maintenance script manually first in dry-run mode:

```bash
cd ~/Yapper
bash scripts/maintain-gh-runner-disk.sh --dry-run
```

Apply the cleanup after reviewing the dry-run output:

```bash
bash scripts/maintain-gh-runner-disk.sh --apply
```

Recommended cron entry for the runner user:

```cron
17 3 * * * /bin/bash /home/runner/Yapper/scripts/maintain-gh-runner-disk.sh --apply >> /home/runner/runner-maintenance.log 2>&1
```

The script keeps the newest Playwright browser builds, prunes stale Playwright/test artifacts older than 14 days, and skips workspace cleanup while an Actions worker process is active.

---

## Linting & Formatting

```bash
make lint    # cargo clippy -D warnings + eslint
make fmt     # cargo fmt + prettier
```

Run before every PR. The CI pipeline enforces both.

---

## Secrets & Security

- Never commit `backend/.env`, `backend/secrets/*.pem`, or `backend/secrets/firebase-service-account.json`
- All three are covered by `.gitignore`
- Production secrets live in Fly.io secret store (`flyctl secrets set KEY=value`)
- R2 credentials are environment variables, never embedded in code

---

## Useful Commands

```bash
# Check health
curl http://localhost:8080/health

# Watch backend logs
make dev-backend 2>&1 | grep -v DEBUG

# Connect to local DB
docker exec -it $(docker ps -qf name=postgres) psql -U yapper -d yapper

# Check sqlx future-incompatibility warnings
cargo report future-incompatibilities

# Tauri build (production)
cd frontend && npm run tauri build
```
