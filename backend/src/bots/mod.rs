/**
 * Bots — bot application management and Discord bot migration.
 *
 * Routes (mounted under /api/v1/bots):
 *   POST   /import-discord  — import a Discord bot via its token → create bot_application + Yapper token
 *   GET    /                — list the caller's bot applications
 *   DELETE /:id             — delete a bot application (revokes all tokens)
 *
 * Token lifecycle:
 *   - Raw token = 32 random bytes encoded as hex (shown ONCE at creation)
 *   - DB stores only SHA-256(raw_token) in bot_tokens.token_hash
 *   - Bot authenticates to the WS/API using the raw token (hashed and compared server-side)
 */
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

// ─── Constants ────────────────────────────────────────────────────────────────

const DISCORD_BOT_API: &str = "https://discord.com/api/users/@me";
/// Maximum bot applications per user.
const MAX_BOTS_PER_USER: i64 = 5;
/// Raw token length in bytes (encoded to 64 hex chars).
const TOKEN_BYTES: usize = 32;

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_bots))
        .route("/import-discord", post(import_discord_bot))
        .route("/:id", delete(delete_bot))
}

// ─── List bots ────────────────────────────────────────────────────────────────

/// GET /api/v1/bots
async fn list_bots(auth: AuthUser, State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT id, name, description, avatar_url, discord_bot_id, created_at
         FROM bot_applications
         WHERE developer_user_id = $1
         ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let bots: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":             r.try_get::<Uuid, _>("id").ok(),
                "name":           r.try_get::<String, _>("name").unwrap_or_default(),
                "description":    r.try_get::<Option<String>, _>("description").ok().flatten(),
                "avatar_url":     r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
                "discord_bot_id": r.try_get::<Option<String>, _>("discord_bot_id").ok().flatten(),
                "created_at":     r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                    .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "bots": bots })))
}

// ─── Import Discord bot ───────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportDiscordBotInput {
    /// The Discord bot token (e.g. "Bot MTxxxxxx.Gyyyy.zzzz")
    discord_token: String,
}

/// POST /api/v1/bots/import-discord
///
/// 1. Calls Discord API with the provided bot token to verify it and fetch bot identity
/// 2. Creates a bot_applications row + a users(bot) stub row
/// 3. Generates a Yapper API token (shown once; stored hashed)
async fn import_discord_bot(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ImportDiscordBotInput>,
) -> AppResult<impl IntoResponse> {
    enforce_bot_limit(auth.user_id, &state).await?;

    let profile = verify_discord_bot_token(&body.discord_token, &state).await?;
    check_not_already_imported(&profile.discord_bot_id, &state).await?;

    let bot_user_id = create_bot_user_stub(&profile, &state).await?;
    let (app_id, raw_token) = create_bot_app_and_token(
        auth.user_id,
        &profile.bot_name,
        &profile.discord_bot_id,
        &state,
    )
    .await?;

    tracing::info!(
        developer_id = %auth.user_id, app_id = %app_id,
        discord_bot_id = %profile.discord_bot_id, "Discord bot imported"
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": app_id, "name": profile.bot_name,
            "discord_bot_id": profile.discord_bot_id, "bot_user_id": bot_user_id,
            "token": raw_token,
            "warning": "Store this token securely. It will not be shown again.",
        })),
    ))
}

struct DiscordBotProfile {
    discord_bot_id: String,
    bot_username: String,
    bot_name: String,
}

async fn enforce_bot_limit(user_id: Uuid, state: &AppState) -> AppResult<()> {
    let count: i64 =
        sqlx::query("SELECT COUNT(*) FROM bot_applications WHERE developer_user_id = $1")
            .bind(user_id)
            .fetch_one(state.db.pool())
            .await
            .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
            .unwrap_or(0);
    if count >= MAX_BOTS_PER_USER {
        return Err(AppError::BadRequest(format!(
            "Maximum {MAX_BOTS_PER_USER} bot applications per user"
        )));
    }
    Ok(())
}

async fn verify_discord_bot_token(token: &str, state: &AppState) -> AppResult<DiscordBotProfile> {
    let token = token.trim();
    let auth_header = if token.starts_with("Bot ") {
        token.to_string()
    } else {
        format!("Bot {token}")
    };

    let resp = state
        .http_client
        .get(DISCORD_BOT_API)
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord API request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::BadRequest(
            "Invalid Discord bot token — Discord API returned an error".into(),
        ));
    }

    let profile: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord profile parse failed: {e}")))?;

    let discord_bot_id = profile["id"]
        .as_str()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No id in Discord bot profile")))?
        .to_string();
    let bot_username = profile["username"]
        .as_str()
        .unwrap_or("imported_bot")
        .to_string();
    let bot_name = format!("{bot_username} (Discord)");

    Ok(DiscordBotProfile {
        discord_bot_id,
        bot_username,
        bot_name,
    })
}

async fn check_not_already_imported(discord_bot_id: &str, state: &AppState) -> AppResult<()> {
    let exists = sqlx::query("SELECT 1 FROM bot_applications WHERE discord_bot_id = $1")
        .bind(discord_bot_id)
        .fetch_optional(state.db.pool())
        .await?
        .is_some();
    if exists {
        return Err(AppError::Conflict(
            "This Discord bot has already been imported".into(),
        ));
    }
    Ok(())
}

async fn create_bot_user_stub(profile: &DiscordBotProfile, state: &AppState) -> AppResult<Uuid> {
    let bot_user_id = Uuid::new_v4();
    let bot_email = format!("bot-{bot_user_id}@bots.yapper.internal");
    let safe_username: String = format!(
        "bot_{}",
        profile
            .bot_username
            .to_lowercase()
            .replace(['-', '.', ' '], "_")
    )
    .chars()
    .take(32)
    .collect();

    sqlx::query(
        "INSERT INTO users (id, email, username, display_name, account_type, discord_id) \
         VALUES ($1, $2, $3, $4, 'bot', $5)",
    )
    .bind(bot_user_id)
    .bind(&bot_email)
    .bind(&safe_username)
    .bind(&profile.bot_username)
    .bind(&profile.discord_bot_id)
    .execute(state.db.pool())
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::Conflict("A bot with this username already exists".into())
        } else {
            AppError::Database(e)
        }
    })?;
    Ok(bot_user_id)
}

async fn create_bot_app_and_token(
    developer_id: Uuid,
    bot_name: &str,
    discord_bot_id: &str,
    state: &AppState,
) -> AppResult<(Uuid, String)> {
    let app_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO bot_applications (id, developer_user_id, name, discord_bot_id) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(app_id)
    .bind(developer_id)
    .bind(bot_name)
    .bind(discord_bot_id)
    .execute(state.db.pool())
    .await?;

    let mut raw_bytes = [0u8; TOKEN_BYTES];
    getrandom::getrandom(&mut raw_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("RNG error: {e}")))?;
    let raw_token: String =
        raw_bytes
            .iter()
            .fold(String::with_capacity(TOKEN_BYTES * 2), |mut s, b| {
                use std::fmt::Write;
                write!(s, "{b:02x}").unwrap();
                s
            });
    let token_hash = format!("{:x}", Sha256::digest(raw_token.as_bytes()));

    sqlx::query("INSERT INTO bot_tokens (bot_app_id, token_hash) VALUES ($1, $2)")
        .bind(app_id)
        .bind(&token_hash)
        .execute(state.db.pool())
        .await?;

    Ok((app_id, raw_token))
}

// ─── Delete bot ───────────────────────────────────────────────────────────────

/// DELETE /api/v1/bots/:id
///
/// Deletes the bot application and all associated tokens.
/// The cascade on bot_tokens (ON DELETE CASCADE) handles token cleanup.
async fn delete_bot(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let result =
        sqlx::query("DELETE FROM bot_applications WHERE id = $1 AND developer_user_id = $2")
            .bind(app_id)
            .bind(auth.user_id)
            .execute(state.db.pool())
            .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Bot application not found or you are not the owner".into(),
        ));
    }

    tracing::info!(developer_id = %auth.user_id, app_id = %app_id, "Bot application deleted");
    Ok(StatusCode::NO_CONTENT)
}
