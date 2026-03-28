use axum::{extract::State, Json};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::LazyLock;
use std::time::Instant;
use uuid::Uuid;

use super::r2;
use crate::{
    auth::AuthUser,
    constants,
    error::{AppError, AppResult},
    AppState,
};

/// Per-user upload rate limiter: stores recent upload timestamps.
static UPLOAD_TIMESTAMPS: LazyLock<DashMap<Uuid, Vec<Instant>>> = LazyLock::new(DashMap::new);

/// Check if the user has exceeded MAX_UPLOADS_PER_MINUTE. Returns Err if so.
fn check_upload_rate(user_id: Uuid) -> AppResult<()> {
    let now = Instant::now();
    let window = std::time::Duration::from_secs(60);
    let max = constants::MAX_UPLOADS_PER_MINUTE as usize;

    let mut entry = UPLOAD_TIMESTAMPS.entry(user_id).or_default();
    // Prune timestamps older than 60s
    entry.retain(|t| now.duration_since(*t) < window);

    if entry.len() >= max {
        return Err(AppError::BadRequest(
            "Upload rate limit exceeded. Try again in a minute.".into(),
        ));
    }

    if entry.is_empty() {
        // All timestamps expired â€” clean up the entry entirely
        // to prevent unbounded growth of empty Vec entries over time.
        drop(entry);
        UPLOAD_TIMESTAMPS.remove(&user_id);
        // Re-insert with the new timestamp
        UPLOAD_TIMESTAMPS.entry(user_id).or_default().push(now);
    } else {
        entry.push(now);
    }
    Ok(())
}

// â”€â”€â”€ Request / Response types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadUrlReq {
    media_type: String,
    /// Client-declared content length in bytes (server enforces tier-based cap).
    content_length: u64,
}

#[derive(Serialize)]
pub struct UploadUrlResp {
    upload_url: String,
    object_key: String,
    /// Seconds until the pre-signed URL expires (informational â€” not enforced here).
    expires_in: u64,
}

// â”€â”€â”€ Handler â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// `POST /api/v2/media/upload-url`
///
/// Generates a pre-signed Cloudflare R2 PUT URL for the client to upload an
/// AES-256-GCMâ€“encrypted media blob directly (no server proxy).
///
/// The client MUST:
///   1. Encrypt the blob client-side with AES-256-GCM before uploading.
///   2. Embed `{ object_key, key, iv }` inside the Signal-encrypted message payload.
///   3. PUT the encrypted blob to `upload_url` with `Content-Type: application/octet-stream`.
///
/// The server never receives the plaintext blob or the AES key.
pub async fn upload_url(
    auth: AuthUser,
    State(_state): State<AppState>,
    Json(req): Json<UploadUrlReq>,
) -> AppResult<Json<UploadUrlResp>> {
    debug_assert!(auth.user_id != Uuid::nil());

    check_upload_rate(auth.user_id)?;

    let media_type = req.media_type.trim().to_lowercase();
    if !r2::ALLOWED_MEDIA_TYPES.contains(&media_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "media_type must be one of: {}",
            r2::ALLOWED_MEDIA_TYPES.join(", ")
        )));
    }

    // Enforce upload size cap based on premium status.
    // Propagate DB errors rather than silently defaulting â€” a transient Neon
    // failure should surface as 500, not silently cap premium users at 10 MB.
    let is_premium: bool = sqlx::query("SELECT is_premium FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(_state.db.pool())
        .await
        .map_err(|e| {
            tracing::error!(user_id = %auth.user_id, "Failed to check premium status: {e}");
            AppError::Internal(e.into())
        })?
        .try_get::<bool, _>("is_premium")
        .unwrap_or(false); // Column may be NULL â€” default to free tier is safe

    let max_size = if is_premium {
        constants::MAX_UPLOAD_SIZE_PREMIUM
    } else {
        constants::MAX_UPLOAD_SIZE
    };

    if req.content_length == 0 {
        return Err(AppError::BadRequest(
            "content_length must be greater than zero".into(),
        ));
    }

    if req.content_length > max_size {
        return Err(AppError::BadRequest(format!(
            "File too large. Maximum upload size is {} MB",
            max_size / (1024 * 1024)
        )));
    }

    let target = r2::generate_upload_url(&media_type, req.content_length).await?;

    // Track the upload for quota/GC (best-effort â€” don't fail the request)
    if let Err(e) = sqlx::query(
        "INSERT INTO media_uploads (user_id, object_key, media_type, size_bytes, expires_at) \
         VALUES ($1, $2, $3, $4, NOW() + ($5::int * INTERVAL '1 day'))",
    )
    .bind(auth.user_id)
    .bind(&target.object_key)
    .bind(&media_type)
    .bind(req.content_length as i64)
    .bind(constants::MEDIA_UPLOAD_EXPIRY_DAYS)
    .execute(_state.db.pool())
    .await
    {
        tracing::warn!(user_id = %auth.user_id, "Failed to track media upload: {e}");
    }

    Ok(Json(UploadUrlResp {
        upload_url: target.upload_url,
        object_key: target.object_key,
        expires_in: 15 * 60,
    }))
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that known-good media types are in the allowed list.
    #[test]
    fn test_allowed_types_include_yap_clip() {
        assert!(r2::ALLOWED_MEDIA_TYPES.contains(&"yap"));
        assert!(r2::ALLOWED_MEDIA_TYPES.contains(&"clip"));
    }

    /// Verify that unknown types would be rejected (mirrors handler logic).
    #[test]
    fn test_unknown_media_type_rejected() {
        let candidate = "audio";
        assert!(!r2::ALLOWED_MEDIA_TYPES.contains(&candidate));
    }
}
