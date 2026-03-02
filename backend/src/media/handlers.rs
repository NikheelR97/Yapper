use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};
use super::r2;

// ─── Request / Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadUrlReq {
    media_type: String,
}

#[derive(Serialize)]
pub struct UploadUrlResp {
    upload_url: String,
    object_key: String,
    /// Seconds until the pre-signed URL expires (informational — not enforced here).
    expires_in: u64,
}

// ─── Handler ─────────────────────────────────────────────────────────────────

/// `POST /api/v1/media/upload-url`
///
/// Generates a pre-signed Cloudflare R2 PUT URL for the client to upload an
/// AES-256-GCM–encrypted media blob directly (no server proxy).
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
    debug_assert!(auth.user_id != uuid::Uuid::nil());

    let media_type = req.media_type.trim().to_lowercase();
    if !r2::ALLOWED_MEDIA_TYPES.contains(&media_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "media_type must be one of: {}",
            r2::ALLOWED_MEDIA_TYPES.join(", ")
        )));
    }

    let target = r2::generate_upload_url(&media_type).await?;

    Ok(Json(UploadUrlResp {
        upload_url: target.upload_url,
        object_key: target.object_key,
        expires_in: 15 * 60,
    }))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

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
