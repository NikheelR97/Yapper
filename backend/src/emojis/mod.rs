/**
 * Emojis — custom server emoji CRUD.
 *
 * Routes (mounted under /api/v1/servers/:server_id/emojis via main.rs wiring):
 *   GET    /api/v1/servers/:id/emojis          — list all emojis for a server
 *   POST   /api/v1/servers/:id/emojis          — upload a new emoji (multipart: name + file)
 *   DELETE /api/v1/servers/:id/emojis/:emo_id  — admin-only delete
 *
 * Upload pipeline:
 *   1. Verify caller is server admin/owner
 *   2. Validate name (2-32 chars, lowercase alphanumeric + underscores)
 *   3. Enforce per-server limit (50 free / 100 premium)
 *   4. Decode image with `image` crate → resize 64×64 → encode as WebP
 *   5. Upload to R2 at  emojis/servers/{server_id}/{emoji_id}.webp
 *   6. Insert DB row + broadcast emoji_added WS event to all server members
 */
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Json, Router,
};
use image::{imageops::FilterType, ImageFormat};
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::Row;
use std::io::Cursor;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    hub::WsOutbound,
    AppState,
};

// ─── Limits ──────────────────────────────────────────────────────────────────

/// Maximum raw upload size accepted (256 KB — before WebP conversion).
const MAX_EMOJI_BYTES: usize = 256 * 1024;
/// Target output canvas for server emojis.
const EMOJI_SIZE: u32 = 64;
/// Free-tier servers may have this many emojis.
const EMOJI_LIMIT_FREE: i64 = 50;
/// Premium-owner servers may have this many emojis.
const EMOJI_LIMIT_PREMIUM: i64 = 100;

static EMOJI_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z0-9_]{2,32}$").unwrap());

// ─── Router ───────────────────────────────────────────────────────────────────

/// Mounted at /api/v1/servers/:server_id/emojis (added in main.rs)
pub fn server_emoji_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_emojis).post(upload_emoji))
        .route("/:emoji_id", delete(delete_emoji))
}

/// Empty stub router still needed for the top-level /api/v1/emojis nest in main.rs.
pub fn router() -> Router<AppState> {
    Router::new()
}

// ─── List ────────────────────────────────────────────────────────────────────

/// GET /api/v1/servers/:server_id/emojis
async fn list_emojis(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    require_server_member(auth.user_id, server_id, &state).await?;

    let rows = sqlx::query(
        "SELECT id, name, image_url, created_by, created_at
         FROM server_emojis
         WHERE server_id = $1
         ORDER BY created_at DESC",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    let emojis: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":         r.try_get::<Uuid, _>("id").ok(),
                "name":       r.try_get::<String, _>("name").unwrap_or_default(),
                "image_url":  r.try_get::<String, _>("image_url").unwrap_or_default(),
                "created_by": r.try_get::<Uuid, _>("created_by").ok(),
                "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "emojis": emojis })))
}

// ─── Upload ──────────────────────────────────────────────────────────────────

/// POST /api/v1/servers/:server_id/emojis
///
/// Multipart fields (order-independent):
///   name  — shortcode name, lowercase alphanumeric + underscores, 2-32 chars
///   file  — raw image bytes (PNG, JPEG, GIF ≤ 256KB)
async fn upload_emoji(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<impl IntoResponse> {
    require_server_admin(auth.user_id, server_id, &state).await?;

    // ── Parse multipart fields ────────────────────────────────────────────────
    let mut name_opt: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        match field.name() {
            Some("name") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("name field error: {e}")))?;
                name_opt = Some(text.trim().to_lowercase());
            }
            Some("file") => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("file field error: {e}")))?;
                if bytes.len() > MAX_EMOJI_BYTES {
                    return Err(AppError::BadRequest(format!(
                        "Image must be at most {} KB",
                        MAX_EMOJI_BYTES / 1024
                    )));
                }
                file_bytes = Some(bytes.to_vec());
            }
            _ => {} // Ignore unknown fields
        }
    }

    let name = name_opt.ok_or_else(|| AppError::BadRequest("Missing `name` field".into()))?;
    let raw_bytes =
        file_bytes.ok_or_else(|| AppError::BadRequest("Missing `file` field".into()))?;

    // ── Validate name ─────────────────────────────────────────────────────────
    if !EMOJI_NAME_REGEX.is_match(&name) {
        return Err(AppError::BadRequest(
            "Emoji name must be 2-32 characters: lowercase letters, digits, underscores".into(),
        ));
    }

    // ── Check name uniqueness within server ───────────────────────────────────
    let name_taken = sqlx::query("SELECT 1 FROM server_emojis WHERE server_id = $1 AND name = $2")
        .bind(server_id)
        .bind(&name)
        .fetch_optional(state.db.pool())
        .await?
        .is_some();

    if name_taken {
        return Err(AppError::Conflict(format!(
            "An emoji named :{name}: already exists in this server"
        )));
    }

    // ── Enforce per-server emoji limit ────────────────────────────────────────
    let limit = emoji_limit_for_server(server_id, &state).await?;
    let current_count: i64 = sqlx::query("SELECT COUNT(*) FROM server_emojis WHERE server_id = $1")
        .bind(server_id)
        .fetch_one(state.db.pool())
        .await
        .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
        .unwrap_or(0);

    if current_count >= limit {
        return Err(AppError::BadRequest(format!(
            "This server has reached its emoji limit ({limit})"
        )));
    }

    // ── Convert to WebP 64×64 ─────────────────────────────────────────────────
    let webp_bytes = tokio::task::spawn_blocking(move || -> AppResult<Vec<u8>> {
        let img = image::load_from_memory(&raw_bytes)
            .map_err(|e| AppError::BadRequest(format!("Unsupported image format: {e}")))?;
        let resized = img.resize_exact(EMOJI_SIZE, EMOJI_SIZE, FilterType::Lanczos3);
        let mut buf = Cursor::new(Vec::new());
        resized
            .write_to(&mut buf, ImageFormat::WebP)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("WebP encode error: {e}")))?;
        Ok(buf.into_inner())
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Spawn blocking error: {e}")))??;

    // ── Upload to R2 ─────────────────────────────────────────────────────────
    let emoji_id = Uuid::new_v4();
    let r2_key = format!("emojis/servers/{server_id}/{emoji_id}.webp");
    let public_url = upload_webp_to_r2(&r2_key, webp_bytes).await?;

    // ── Insert DB row ─────────────────────────────────────────────────────────
    sqlx::query(
        "INSERT INTO server_emojis (id, server_id, name, image_url, image_r2_key, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(emoji_id)
    .bind(server_id)
    .bind(&name)
    .bind(&public_url)
    .bind(&r2_key)
    .bind(auth.user_id)
    .execute(state.db.pool())
    .await?;

    // ── Broadcast emoji_added to all server members ───────────────────────────
    let emoji_payload = serde_json::json!({
        "id":       emoji_id,
        "name":     name,
        "image_url": public_url,
        "server_id": server_id,
        "created_by": auth.user_id,
    });
    broadcast_to_server_members(
        server_id,
        serde_json::json!({ "type": "emoji_added", "emoji": emoji_payload }),
        &state,
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id":         emoji_id,
            "name":       &name,
            "image_url":  public_url,
            "server_id":  server_id,
        })),
    ))
}

// ─── Delete ──────────────────────────────────────────────────────────────────

/// DELETE /api/v1/servers/:server_id/emojis/:emoji_id
async fn delete_emoji(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((server_id, emoji_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    require_server_admin(auth.user_id, server_id, &state).await?;

    // Fetch the R2 key before deleting so we can clean up from storage
    let row = sqlx::query(
        "DELETE FROM server_emojis WHERE id = $1 AND server_id = $2 RETURNING image_r2_key, name",
    )
    .bind(emoji_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Emoji not found".into()))?;

    let r2_key: String = row.try_get("image_r2_key").unwrap_or_default();
    let name: String = row.try_get("name").unwrap_or_default();

    // Best-effort R2 deletion — log but don't fail the request if R2 is down
    if let Err(e) = delete_from_r2(&r2_key).await {
        tracing::warn!(emoji_id = %emoji_id, r2_key = %r2_key, "Failed to delete emoji from R2: {e}");
    }

    // Broadcast emoji_removed
    broadcast_to_server_members(
        server_id,
        serde_json::json!({
            "type":      "emoji_removed",
            "emoji_id":  emoji_id,
            "name":      name,
            "server_id": server_id,
        }),
        &state,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ─── R2 helpers ───────────────────────────────────────────────────────────────

/// Upload raw WebP bytes to R2 and return the public HTTPS URL.
///
/// Requires the R2 client to have been initialised at startup (via `media::init_r2()`).
/// Falls back gracefully if R2 is not configured (dev / CI environments).
async fn upload_webp_to_r2(r2_key: &str, webp_bytes: Vec<u8>) -> AppResult<String> {
    use aws_sdk_s3::primitives::ByteStream;

    let client = crate::media::r2::r2_client_opt();
    let bucket = crate::media::r2::r2_bucket_opt();

    let (Some(client), Some(bucket)) = (client, bucket) else {
        // R2 not configured — return a placeholder URL for local dev
        tracing::warn!(
            r2_key,
            "R2 not configured; returning stub URL for emoji upload"
        );
        return Ok(format!("https://cdn.example.com/{r2_key}"));
    };

    client
        .put_object()
        .bucket(bucket)
        .key(r2_key)
        .content_type("image/webp")
        .body(ByteStream::from(webp_bytes))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("R2 put_object error: {e}");
            AppError::Internal(anyhow::anyhow!("Failed to upload emoji to R2: {e}"))
        })?;

    // Construct public URL from the R2 public custom domain if set, else fall back to bucket URL
    let public_base =
        std::env::var("R2_PUBLIC_URL").unwrap_or_else(|_| format!("https://pub.r2.dev/{bucket}"));
    Ok(format!("{public_base}/{r2_key}"))
}

/// Delete an object from R2. Returns an error on failure (caller decides whether to propagate).
async fn delete_from_r2(r2_key: &str) -> anyhow::Result<()> {
    let client = crate::media::r2::r2_client_opt();
    let bucket = crate::media::r2::r2_bucket_opt();

    let (Some(client), Some(bucket)) = (client, bucket) else {
        return Ok(()); // Nothing to do if R2 not configured
    };

    client
        .delete_object()
        .bucket(bucket)
        .key(r2_key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("R2 delete_object error: {e}"))?;

    Ok(())
}

// ─── Authorisation helpers ────────────────────────────────────────────────────

async fn require_server_member(user_id: Uuid, server_id: Uuid, state: &AppState) -> AppResult<()> {
    let is_member =
        sqlx::query("SELECT 1 FROM server_memberships WHERE user_id = $1 AND server_id = $2")
            .bind(user_id)
            .bind(server_id)
            .fetch_optional(state.db.pool())
            .await?
            .is_some();

    if !is_member {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn require_server_admin(user_id: Uuid, server_id: Uuid, state: &AppState) -> AppResult<()> {
    let row =
        sqlx::query("SELECT role FROM server_memberships WHERE user_id = $1 AND server_id = $2")
            .bind(user_id)
            .bind(server_id)
            .fetch_optional(state.db.pool())
            .await?
            .ok_or(AppError::Forbidden)?;

    let role: String = row.try_get("role").unwrap_or_default();
    if !matches!(role.as_str(), "owner" | "admin") {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Returns the emoji limit for a server, based on whether its owner has premium.
async fn emoji_limit_for_server(server_id: Uuid, state: &AppState) -> AppResult<i64> {
    let row = sqlx::query(
        "SELECT u.is_premium
         FROM servers s
         JOIN users u ON u.id = s.owner_id
         WHERE s.id = $1 AND u.deleted_at IS NULL",
    )
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    let is_premium = row
        .and_then(|r| r.try_get::<bool, _>("is_premium").ok())
        .unwrap_or(false);

    Ok(if is_premium {
        EMOJI_LIMIT_PREMIUM
    } else {
        EMOJI_LIMIT_FREE
    })
}

// ─── WS broadcast helper ─────────────────────────────────────────────────────

/// Fan out a JSON payload to all members of a server over WebSocket.
async fn broadcast_to_server_members(
    server_id: Uuid,
    payload: serde_json::Value,
    state: &AppState,
) {
    let rows = sqlx::query("SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT 500")
        .bind(server_id)
        .fetch_all(state.db.pool())
        .await;

    let Ok(rows) = rows else { return };

    let member_ids: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
        .collect();

    state
        .hub
        .broadcast(&member_ids, WsOutbound::Message { payload });
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_name_accepts_valid_names() {
        assert!(EMOJI_NAME_REGEX.is_match("fire"));
        assert!(EMOJI_NAME_REGEX.is_match("party_blob"));
        assert!(EMOJI_NAME_REGEX.is_match("sparkle_123"));
        assert!(EMOJI_NAME_REGEX.is_match("ab")); // minimum 2 chars
    }

    #[test]
    fn emoji_name_rejects_invalid_names() {
        // Too short
        assert!(!EMOJI_NAME_REGEX.is_match("a"));
        // Uppercase
        assert!(!EMOJI_NAME_REGEX.is_match("Fire"));
        // Special characters
        assert!(!EMOJI_NAME_REGEX.is_match("fire!"));
        assert!(!EMOJI_NAME_REGEX.is_match("fire-cat"));
        // Too long (33 chars)
        assert!(!EMOJI_NAME_REGEX.is_match(&"a".repeat(33)));
    }

    #[test]
    fn emoji_limits_differ_by_tier() {
        assert!(EMOJI_LIMIT_FREE < EMOJI_LIMIT_PREMIUM);
        assert_eq!(EMOJI_LIMIT_FREE, 50);
        assert_eq!(EMOJI_LIMIT_PREMIUM, 100);
    }

    #[test]
    fn webp_conversion_produces_correct_dimensions() {
        // Create a small test PNG (8×8 red square) using image crate
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255u8, 0, 0, 255]));
        let mut buf = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .resize_exact(EMOJI_SIZE, EMOJI_SIZE, FilterType::Lanczos3)
            .write_to(&mut buf, ImageFormat::WebP)
            .expect("WebP encode should succeed");

        let webp_bytes = buf.into_inner();
        assert!(!webp_bytes.is_empty(), "WebP output must not be empty");

        // Decode back and verify dimensions
        let decoded = image::load_from_memory(&webp_bytes).expect("WebP should decode");
        assert_eq!(decoded.width(), EMOJI_SIZE);
        assert_eq!(decoded.height(), EMOJI_SIZE);
    }
}
