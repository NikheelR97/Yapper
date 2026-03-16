/**
 * Discord — import a Discord profile into an already-authenticated Yapper account.
 *
 * Routes (mounted under /api/v1/discord):
 *   GET  /import-profile            — initiates Discord OAuth (scope: identify) for profile import
 *   GET  /import-profile/callback   — exchanges code, downloads avatar, re-uploads to R2,
 *                                     updates users row with discord_id + avatar_url
 *
 * This flow is separate from the new-user OAuth in auth/ — it requires an existing session.
 */
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::time::Instant;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

/// Discord import states older than this are considered expired.
const IMPORT_STATE_TTL_SECS: u64 = 600; // 10 minutes

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/import-profile", get(import_profile_start))
        .route("/import-profile/callback", get(import_profile_callback))
}

// ─── Config ───────────────────────────────────────────────────────────────────

/// Discord OAuth2 endpoints and required scope for profile import.
const DISCORD_AUTH_URL: &str = "https://discord.com/api/oauth2/authorize";
const DISCORD_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const DISCORD_API_ME: &str = "https://discord.com/api/users/@me";
/// Import flow redirects to a dedicated path so auth/ callback isn't polluted.
const REDIRECT_SUFFIX: &str = "/api/v1/discord/import-profile/callback";

// ─── Step 1: Initiate OAuth ───────────────────────────────────────────────────

/// GET /api/v1/discord/import-profile
///
/// Redirects the logged-in user to Discord's OAuth consent screen.
/// An opaque CSRF token is used as the `state` param; the corresponding
/// user_id is stored server-side in `discord_import_states` so it never
/// appears in the URL.
async fn import_profile_start(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let client_id = std::env::var("DISCORD_CLIENT_ID")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("DISCORD_CLIENT_ID not set")))?;
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Generate random 16-byte CSRF token (hex)
    let mut csrf_bytes = [0u8; 16];
    getrandom::getrandom(&mut csrf_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("RNG error: {e}")))?;
    let csrf_token: String = csrf_bytes
        .iter()
        .fold(String::with_capacity(32), |mut s, b| {
            use std::fmt::Write;
            write!(s, "{b:02x}").unwrap();
            s
        });

    // Store csrf_token → (user_id, timestamp) server-side.
    // The URL state param is the opaque token only — user_id never appears in the URL.
    state
        .discord_import_states
        .insert(csrf_token.clone(), (auth.user_id, Instant::now()));

    let redirect_uri = format!("{base_url}{REDIRECT_SUFFIX}");
    let auth_url = format!(
        "{DISCORD_AUTH_URL}?client_id={client_id}&redirect_uri={redirect_uri}\
         &response_type=code&scope=identify&state={csrf_token}"
    );

    Ok(Redirect::to(&auth_url))
}

// ─── Step 2: Handle callback ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct DiscordCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// GET /api/v1/discord/import-profile/callback
///
/// 1. Validate state token (CSRF guard)
/// 2. Exchange code for Discord access token
/// 3. Fetch Discord profile (id, username, avatar hash)
/// 4. Download avatar PNG from Discord CDN → re-upload to R2
/// 5. Update users row (discord_id, avatar_url)
async fn import_profile_callback(
    State(state): State<AppState>,
    Query(query): Query<DiscordCallbackQuery>,
) -> AppResult<impl IntoResponse> {
    // User denied consent or Discord returned an error
    if let Some(err) = query.error {
        return Err(AppError::BadRequest(format!("Discord OAuth error: {err}")));
    }

    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest("Missing code".into()))?;
    let csrf_token = query
        .state
        .ok_or_else(|| AppError::BadRequest("Missing state".into()))?;

    // Validate CSRF state and retrieve user_id from server-side store
    let user_id = validate_import_state(&csrf_token, &state)?;

    // Exchange authorization code for access token
    let client_id = std::env::var("DISCORD_CLIENT_ID")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("DISCORD_CLIENT_ID not set")))?;
    let client_secret = std::env::var("DISCORD_CLIENT_SECRET")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("DISCORD_CLIENT_SECRET not set")))?;
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let redirect_uri = format!("{base_url}{REDIRECT_SUFFIX}");

    let http_client = reqwest::Client::new();

    let token_resp: serde_json::Value = http_client
        .post(DISCORD_TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord token request failed: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord token parse failed: {e}")))?;

    let access_token = token_resp["access_token"]
        .as_str()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No access_token in Discord response")))?
        .to_string();

    // Fetch Discord profile
    let profile: serde_json::Value = http_client
        .get(DISCORD_API_ME)
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord profile fetch failed: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord profile parse failed: {e}")))?;

    let discord_id = profile["id"]
        .as_str()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No id in Discord profile")))?
        .to_string();

    // Check if this Discord account is already linked to a different Yapper user
    let already_linked = sqlx::query(
        "SELECT id FROM users WHERE discord_id = $1 AND id != $2 AND deleted_at IS NULL",
    )
    .bind(&discord_id)
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?
    .is_some();

    if already_linked {
        return Err(AppError::Conflict(
            "This Discord account is already linked to another Yapper account".into(),
        ));
    }

    // Download and re-upload Discord avatar to R2 (Discord CDN URLs are not permanent)
    let avatar_url = if let (Some(avatar_hash), Some(discord_id_str)) =
        (profile["avatar"].as_str(), profile["id"].as_str())
    {
        let cdn_url = format!(
            "https://cdn.discordapp.com/avatars/{discord_id_str}/{avatar_hash}.png?size=256"
        );
        match download_and_reupload_avatar(&http_client, &cdn_url, user_id).await {
            Ok(url) => Some(url),
            Err(e) => {
                tracing::warn!(user_id = %user_id, "Failed to re-upload Discord avatar: {e}");
                None
            }
        }
    } else {
        None
    };

    // Update user record
    sqlx::query(
        "UPDATE users SET discord_id = $2, avatar_url = COALESCE($3, avatar_url)
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(&discord_id)
    .bind(avatar_url.as_deref())
    .execute(state.db.pool())
    .await?;

    tracing::info!(user_id = %user_id, discord_id = %discord_id, "Discord profile imported");

    Ok(Json(serde_json::json!({
        "status":     "ok",
        "discord_id": discord_id,
        "avatar_url": avatar_url,
    })))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn validate_import_state(csrf_token: &str, state: &AppState) -> AppResult<Uuid> {
    // Consume the stored entry (one-time use).
    let (_, (user_id, created_at)) = state
        .discord_import_states
        .remove(csrf_token)
        .ok_or_else(|| AppError::BadRequest("Invalid or expired OAuth state".into()))?;

    // Reject tokens that are too old.
    if created_at.elapsed().as_secs() > IMPORT_STATE_TTL_SECS {
        return Err(AppError::BadRequest("OAuth state has expired".into()));
    }

    Ok(user_id)
}

/// Downloads an image from a URL, converts it to a 256×256 WebP, and uploads it to R2.
/// Returns the public URL of the uploaded image.
async fn download_and_reupload_avatar(
    client: &reqwest::Client,
    url: &str,
    user_id: Uuid,
) -> anyhow::Result<String> {
    const MAX_AVATAR_BYTES: usize = 5 * 1024 * 1024; // 5 MB

    let resp = client.get(url).send().await?;
    let content_length = resp.content_length().unwrap_or(0) as usize;
    if content_length > MAX_AVATAR_BYTES {
        anyhow::bail!("Avatar download too large: {content_length} bytes");
    }
    let image_bytes = resp.bytes().await?;
    if image_bytes.len() > MAX_AVATAR_BYTES {
        anyhow::bail!("Avatar download too large: {} bytes", image_bytes.len());
    }
    let image_bytes = image_bytes.to_vec();

    // Resize to 256×256 WebP in a blocking thread
    let webp_bytes = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        use image::{imageops::FilterType, ImageFormat};
        use std::io::Cursor;

        let img = image::load_from_memory(&image_bytes)?;
        let resized = img.resize_exact(256, 256, FilterType::Lanczos3);
        let mut buf = Cursor::new(Vec::new());
        resized.write_to(&mut buf, ImageFormat::WebP)?;
        Ok(buf.into_inner())
    })
    .await??;

    let r2_key = format!("avatars/{user_id}.webp");

    let client_opt = crate::media::r2::r2_client_opt();
    let bucket_opt = crate::media::r2::r2_bucket_opt();

    let (Some(r2), Some(bucket)) = (client_opt, bucket_opt) else {
        // R2 not configured in dev
        return Ok(format!("https://cdn.example.com/{r2_key}"));
    };

    use aws_sdk_s3::primitives::ByteStream;
    r2.put_object()
        .bucket(bucket)
        .key(&r2_key)
        .content_type("image/webp")
        .body(ByteStream::from(webp_bytes))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("R2 avatar upload error: {e}"))?;

    let public_base =
        std::env::var("R2_PUBLIC_URL").unwrap_or_else(|_| format!("https://pub.r2.dev/{bucket}"));
    Ok(format!("{public_base}/{r2_key}"))
}
