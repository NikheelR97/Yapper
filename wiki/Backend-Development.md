# Backend Development

The backend is a single Rust binary (`yapper-server`) combining the HTTP API and WebSocket hub.

## Project structure

```
backend/src/
├── main.rs           # Server bootstrap, router assembly
├── db.rs             # Database pool wrapper (sqlx + migrations)
├── error.rs          # AppError enum → HTTP status codes
├── hub.rs            # In-memory WebSocket hub
├── csrf.rs           # CSRF double-submit middleware
├── auth/             # Register · login · refresh · OAuth (Discord · Google)
├── users/            # Profiles · follow graph · GDPR export · account delete
├── servers/          # Servers · memberships · invite links
├── channels/         # Channel CRUD
├── messages/         # DM + channel messages (v1 + v2 envelope format)
├── keys/             # Signal key server (identity · signed prekey · OPKs)
├── devices/          # Multi-device registration and trust
├── media/            # Presigned R2 upload URLs
├── canvas/           # Live canvas (music · polls · clips)
├── explore/          # Server search · trending tags · communities
├── emojis/           # Custom emoji upload (WebP conversion)
├── parental/         # COPPA child accounts · parent approval workflows
├── screentime/       # Screen time report ingestion + parent dashboard
├── bots/             # Discord bot import
├── discord/          # Discord profile import
├── premium/          # Stripe webhook · promo codes · subscription status
├── notifications/    # FCM device token registration
└── support/          # User support tickets → HubSpot CRM
```

## API versioning

```
/health                     — unauthenticated health check
/ws                         — WebSocket upgrade
/auth/oauth/:provider       — OAuth redirect (Discord · Google)
/api/v2/…                   — current REST API
/api/v2/auth/…              — multi-device auth (login · refresh · logout)
/api/v2/devices/…           — device management
/api/v2/keys/…              — Signal key distribution (v2 envelope)
/api/v2/conversations/…     — DM v2 (envelope + message number)
```

## Adding a new module

1. Create `src/your_module/mod.rs`
2. Add a `pub fn router() -> Router<AppState>` that returns all routes
3. Register in `main.rs`:
   ```rust
   mod your_module;
   // in api_router():
   .nest("/your_module", your_module::router())
   ```
4. Create a migration if new tables are needed: `sqlx migrate add your_feature`
5. Run `cargo sqlx prepare` to update the offline query cache

## Database patterns

The default is `sqlx::query()` with `.bind()` and `.try_get()`. This is used throughout the codebase and provides parameterised query safety without requiring `cargo sqlx prepare` after every schema change:

```rust
let row = sqlx::query("SELECT id, username FROM users WHERE id = $1")
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await?;
let username: String = row.try_get("username")?;
```

The compile-time `sqlx::query!()` macro is optional for high-stability hot paths where an extra layer of schema-drift detection is valuable. Both approaches are acceptable — they produce identical parameterised SQL at runtime.

## Error handling

`AppError` in `error.rs` covers all common cases:

```rust
return Err(AppError::NotFound("User not found".into()));
return Err(AppError::Forbidden);
return Err(AppError::BadRequest("Invalid input".into()));
return Err(AppError::Conflict("Username already taken".into()));
return Err(AppError::Database(e));          // sqlx::Error
return Err(AppError::Internal(anyhow!(e))); // unexpected
```

Each variant maps to the appropriate HTTP status code automatically via `IntoResponse`.

## Authentication

All protected routes use the `AuthUser` extractor:

```rust
async fn my_handler(
    auth: AuthUser,           // extracts user_id from JWT Bearer token
    State(state): State<AppState>,
    Json(body): Json<MyInput>,
) -> AppResult<impl IntoResponse> {
    // auth.user_id is a Uuid
}
```

The JWT is RS256-signed. The public key is loaded at startup. Tokens expire after 15 minutes; clients use the refresh token (HttpOnly cookie) to obtain new access tokens.

## WebSocket messages

All WebSocket messages are JSON with a `type` field. Outbound messages from the server:

| Type | Payload |
|------|---------|
| `new_message` | message envelope |
| `typing` | `{ channel_id, user_id }` |
| `typing_stop` | `{ channel_id, user_id }` |
| `read_receipt` | `{ message_id, user_id, read_at }` |
| `presence_update` | `{ user_id, status }` |
| `canvas_update` | canvas patch |
| `emoji_added` | `{ server_id, emoji }` |
| `sender_key_dist` | Signal sender key distribution |

## Coding conventions

We follow a subset of the NASA/JPL Power of Ten rules adapted for Rust:

- **Rule 1** — No `goto`, no unconditional loops. Use iterators.
- **Rule 3** — Validate all inputs at module boundaries. Define `MAX_*` constants.
- **Rule 4** — No function body exceeds 60 executable lines. Extract helpers.
- **Rule 5** — Assert preconditions. Return `AppError::BadRequest` on bad input.
- **Rule 7** — Check every `Result` and `Option`. No `unwrap()` in production paths.
- **Rule 10** — Minimise unsafe. Zero `unsafe` blocks currently.

## Running clippy and fmt

```bash
cd backend
cargo fmt
cargo clippy -- -D warnings
```

CI enforces both with `cargo fmt --check` and `cargo clippy -- -D warnings`.
