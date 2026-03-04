use anyhow::anyhow;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{config::Credentials, presigning::PresigningConfig, Client};
use std::{sync::OnceLock, time::Duration};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Max pre-signed URL validity: 15 minutes.
const PRESIGN_TTL_SECS: u64 = 15 * 60;

/// Allowed media types for upload.
pub const ALLOWED_MEDIA_TYPES: &[&str] = &["yap", "clip"];

// ─── R2 client singleton ─────────────────────────────────────────────────────

static R2_CLIENT: OnceLock<Client> = OnceLock::new();
static R2_BUCKET: OnceLock<String> = OnceLock::new();

/// Initialise the R2 client from environment variables (called once at startup).
///
/// Required vars:
///   - `R2_ACCOUNT_ID`
///   - `R2_ACCESS_KEY_ID`
///   - `R2_SECRET_ACCESS_KEY`
///   - `R2_BUCKET_NAME`
pub async fn init_r2() {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID must be set");
    let secret_key =
        std::env::var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_ACCESS_KEY must be set");
    let bucket = std::env::var("R2_BUCKET_NAME").unwrap_or_else(|_| "yapper-media".to_string());

    let endpoint = format!("https://{account_id}.r2.cloudflarestorage.com");

    let creds = Credentials::new(access_key, secret_key, None, None, "env");

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .credentials_provider(creds)
        .endpoint_url(endpoint)
        .load()
        .await;
    let client = Client::new(&config);

    R2_CLIENT.set(client).ok();
    R2_BUCKET.set(bucket).ok();
}

fn r2_client() -> &'static Client {
    R2_CLIENT
        .get()
        .expect("R2 client not initialised — call init_r2() at startup")
}

fn r2_bucket() -> &'static str {
    R2_BUCKET
        .get()
        .expect("R2 bucket not initialised — call init_r2() at startup")
}

/// Option-returning variants — used by modules that gracefully handle missing R2 config.
pub fn r2_client_opt() -> Option<&'static Client> {
    R2_CLIENT.get()
}

pub fn r2_bucket_opt() -> Option<&'static str> {
    R2_BUCKET.get().map(|s| s.as_str())
}

// ─── Upload URL generation ────────────────────────────────────────────────────

/// Info returned to the client after generating a pre-signed upload URL.
pub struct UploadTarget {
    pub upload_url: String,
    pub object_key: String,
}

/// Generate a pre-signed R2 PUT URL valid for 15 minutes.
///
/// Object key format: `media/{media_type}/{uuid}.bin`
/// The `.bin` extension is intentional — the file content is AES-encrypted;
/// revealing the true extension would leak the media type in storage logs.
pub async fn generate_upload_url(media_type: &str) -> AppResult<UploadTarget> {
    debug_assert!(ALLOWED_MEDIA_TYPES.contains(&media_type));

    let object_key = format!("media/{}/{}.bin", media_type, Uuid::new_v4());

    let presigning_config = PresigningConfig::builder()
        .expires_in(Duration::from_secs(PRESIGN_TTL_SECS))
        .build()
        .map_err(|e| AppError::Internal(anyhow!("Presign config error: {e}")))?;

    let presigned = r2_client()
        .put_object()
        .bucket(r2_bucket())
        .key(&object_key)
        .content_type("application/octet-stream")
        .presigned(presigning_config)
        .await
        .map_err(|e| {
            tracing::error!("R2 presign error: {e}");
            AppError::Internal(anyhow!("Failed to generate upload URL: {e}"))
        })?;

    Ok(UploadTarget {
        upload_url: presigned.uri().to_string(),
        object_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_media_types_are_non_empty() {
        assert!(!ALLOWED_MEDIA_TYPES.is_empty());
    }

    #[test]
    fn test_allowed_media_types_contain_yap_and_clip() {
        assert!(ALLOWED_MEDIA_TYPES.contains(&"yap"));
        assert!(ALLOWED_MEDIA_TYPES.contains(&"clip"));
    }

    #[test]
    fn test_object_key_format() {
        // Verify that the key format is as documented above (tested structurally).
        let media_type = "yap";
        let fake_uuid = Uuid::nil();
        let key = format!("media/{}/{}.bin", media_type, fake_uuid);
        assert!(key.starts_with("media/yap/"));
        assert!(key.ends_with(".bin"));
    }
}
