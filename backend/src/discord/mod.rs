/**
 * Discord Ã¢â‚¬â€ import a Discord profile into an already-authenticated Yapper account.
 *
 * Routes (mounted under /api/v2/discord):
 *   GET  /import-profile            Ã¢â‚¬â€ initiates Discord OAuth (scope: identify) for profile import
 *   GET  /import-profile/callback   Ã¢â‚¬â€ exchanges code, downloads avatar, re-uploads to R2,
 *                                     updates users row with normalized linked identity + avatar_url
 *
 * This flow is separate from the new-user OAuth in auth/ Ã¢â‚¬â€ it requires an existing session.
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
    auth::{oauth::ensure_linked_identity, AuthUser},
    error::{AppError, AppResult},
    AppState,
};

/// Discord import states older than this are considered expired.
const IMPORT_STATE_TTL_SECS: u64 = 600; // 10 minutes

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬ Router Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/import-profile", get(import_profile_start))
        .route("/import-profile/callback", get(import_profile_callback))
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬ Config Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Discord OAuth2 endpoints and required scope for profile import.
const DISCORD_AUTH_URL: &str = "https://discord.com/api/oauth2/authorize";
const DISCORD_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const DISCORD_API_ME: &str = "https://discord.com/api/users/@me";
/// Import flow redirects to a dedicated path so auth/ callback isn't polluted.
const REDIRECT_SUFFIX: &str = "/api/v2/discord/import-profile/callback";

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬ Step 1: Initiate OAuth Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// GET /api/v2/discord/import-profile
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
    let csrf_token: String = crate::hex_encode(&csrf_bytes);

    // Store csrf_token Ã¢â€ â€™ (user_id, timestamp) server-side.
    // The URL state param is the opaque token only Ã¢â‚¬â€ user_id never appears in the URL.
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

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬ Step 2: Handle callback Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

#[derive(Deserialize)]
struct DiscordCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// GET /api/v2/discord/import-profile/callback
///
/// 1. Validate state token (CSRF guard)
/// 2. Exchange code for Discord access token
/// 3. Fetch Discord profile (id, username, avatar hash)
/// 4. Download avatar PNG from Discord CDN Ã¢â€ â€™ re-upload to R2
/// 5. Update users row (linked identity, avatar_url)
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

    let http_client = &state.http_client;

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

    // Download and re-upload Discord avatar to R2 (Discord CDN URLs are not permanent)
    let avatar_url = if let (Some(avatar_hash), Some(discord_id_str)) =
        (profile["avatar"].as_str(), profile["id"].as_str())
    {
        let cdn_url = format!(
            "https://cdn.discordapp.com/avatars/{discord_id_str}/{avatar_hash}.png?size=256"
        );
        match download_and_reupload_avatar(http_client, &cdn_url, user_id).await {
            Ok(url) => Some(url),
            Err(e) => {
                tracing::warn!(user_id = %user_id, "Failed to re-upload Discord avatar: {e}");
                None
            }
        }
    } else {
        None
    };

    let mut tx = state.db.pool().begin().await?;

    let already_linked = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT uli.user_id
        FROM user_linked_identities uli
        JOIN users u ON u.id = uli.user_id
        WHERE uli.provider = 'discord'
          AND uli.provider_subject = $1
          AND u.deleted_at IS NULL
        "#,
    )
    .bind(&discord_id)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(linked_user_id) = already_linked {
        if linked_user_id != user_id {
            tx.rollback().await.ok();
            return Err(AppError::Conflict(
                "This Discord account is already linked to another Yapper account".into(),
            ));
        }
    }

    ensure_linked_identity(&mut tx, user_id, "discord", &discord_id).await?;

    sqlx::query(
        "UPDATE users SET avatar_url = COALESCE($2, avatar_url) WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(avatar_url.as_deref())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(user_id = %user_id, discord_id = %discord_id, "Discord profile imported");
    Ok(Json(serde_json::json!({
        "status":     "ok",
        "discord_id": discord_id,
        "avatar_url": avatar_url,
    })))
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬ Helpers Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

/// Downloads an image from a URL, converts it to a 256Ãƒâ€”256 WebP, and uploads it to R2.
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

    // Validate dimensions BEFORE full decompression to prevent decompression bombs.
    // A malicious Discord avatar could be a tiny file with huge pixel dimensions.
    {
        use image::io::Reader as ImageReader;
        use std::io::Cursor;
        let reader = ImageReader::new(Cursor::new(&image_bytes))
            .with_guessed_format()
            .map_err(|e| anyhow::anyhow!("Unrecognised image format: {e}"))?;
        let (w, h) = reader
            .into_dimensions()
            .map_err(|e| anyhow::anyhow!("Cannot read image dimensions: {e}"))?;
        if w > crate::constants::MAX_IMAGE_DIMENSION || h > crate::constants::MAX_IMAGE_DIMENSION {
            anyhow::bail!("Discord avatar dimensions {w}Ãƒâ€”{h} exceed limit");
        }
        if w.saturating_mul(h) > crate::constants::MAX_DECODED_PIXELS {
            anyhow::bail!("Discord avatar pixel count exceeds limit");
        }
    }

    // Resize to 256Ãƒâ€”256 WebP in a blocking thread
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
