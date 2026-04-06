use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, RateLimiter};
use sqlx::Row;
use std::num::NonZeroU32;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    devices::DeviceTrustState,
    error::{AppError, AppResult},
    AppState,
};

pub type KeyedLimiter<T> = RateLimiter<T, DefaultKeyedStateStore<T>, DefaultClock>;

pub static BACKUP_RETRIEVE_LIMITER: once_cell::sync::Lazy<KeyedLimiter<Uuid>> =
    once_cell::sync::Lazy::new(|| {
        RateLimiter::keyed(
            governor::Quota::with_period(std::time::Duration::from_secs(60 * 60))
                .expect("valid backup quota")
                .allow_burst(NonZeroU32::new(5).expect("non-zero burst")),
        )
    });

/// Resolve the server-authoritative `signal_device_id` for the authenticated
/// user's device. The shared upload helpers use this value so that only the
/// authenticated device can publish its own Signal material.
///
/// Bot accounts have no device, so they are rejected outright — bots do not
/// participate in the Signal protocol.
pub async fn resolve_signal_device_id(auth: &AuthUser, state: &AppState) -> AppResult<i32> {
    let device_uuid = auth.device_id.ok_or_else(|| {
        AppError::BadRequest(
            "Key endpoints require a device-bound token. Use /api/v2/auth/* to authenticate."
                .to_string(),
        )
    })?;
    let device = crate::devices::get_device_for_user(auth.user_id, device_uuid, state).await?;
    if device.revoked_at.is_some() || device.trust_state == DeviceTrustState::Revoked {
        return Err(AppError::Unauthorized);
    }
    if device.trust_state != DeviceTrustState::Trusted {
        return Err(AppError::Forbidden);
    }
    Ok(device.signal_device_id)
}

/// Reject all-zero X25519 keys, which lie in a small subgroup and would
/// produce predictable shared secrets.
pub fn reject_trivial_x25519_key(bytes: &[u8], field: &str) -> AppResult<()> {
    if bytes.iter().all(|byte| *byte == 0) {
        return Err(AppError::BadRequest(format!(
            "{field} must not be all-zero"
        )));
    }
    Ok(())
}

/// Verify that the signed prekey's Ed25519 signature was produced by the
/// device's stored signing key. Prevents clients from uploading arbitrary
/// prekeys that they cannot prove ownership of.
///
/// # E2EE contract
///
/// * The server never sees private keys. It only verifies that the public
///   signed prekey was signed by the Ed25519 identity key the device previously
///   uploaded. This prevents a compromised server from substituting prekeys.
pub async fn verify_signed_prekey_signature(
    state: &AppState,
    user_id: Uuid,
    device_id: i32,
    public_key: &[u8],
    signature: &[u8],
) -> AppResult<()> {
    let identity_row = sqlx::query(
        r#"
        SELECT signing_key
        FROM identity_keys
        WHERE user_id = $1 AND device_id = $2
        "#,
    )
    .bind(user_id)
    .bind(device_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::BadRequest("Upload identity key before signed prekeys".into()))?;

    let signing_key: Vec<u8> = identity_row.try_get("signing_key")?;
    let signing_key_bytes: [u8; 32] = signing_key
        .try_into()
        .map_err(|_| AppError::BadRequest("Stored signing key must be 32 bytes".into()))?;
    let signature_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| AppError::BadRequest("signature must be 64 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&signing_key_bytes)
        .map_err(|_| AppError::BadRequest("Stored signing key is invalid".into()))?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(public_key, &signature)
        .map_err(|_| AppError::BadRequest("Signed prekey signature is invalid".into()))
}

pub fn parse_device_ids_filter(value: Option<&str>) -> AppResult<Option<Vec<Uuid>>> {
    let Some(value) = value else {
        return Ok(None);
    };

    let parsed = value
        .split(',')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            Uuid::parse_str(segment)
                .map_err(|_| AppError::BadRequest(format!("Invalid device_id {segment}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if parsed.len() > crate::constants::MAX_DEVICE_IDS {
        return Err(AppError::BadRequest(format!(
            "Too many device_ids (max {})",
            crate::constants::MAX_DEVICE_IDS
        )));
    }

    if parsed.is_empty() {
        return Err(AppError::BadRequest(
            "device_ids must include at least one UUID".into(),
        ));
    }

    Ok(Some(parsed))
}

pub fn backup_restore_message(current_device_id: Uuid, source_device_id: Uuid) -> Vec<u8> {
    format!("yapper-backup-restore:v1:{current_device_id}:{source_device_id}").into_bytes()
}

pub async fn verify_backup_restore_signature(
    state: &AppState,
    user_id: Uuid,
    current_device_id: Uuid,
    source_device_id: Uuid,
    source_signal_device_id: i32,
    signature_b64: &str,
) -> AppResult<()> {
    let signature_bytes = BASE64
        .decode(signature_b64)
        .map_err(|_| AppError::BadRequest("Invalid source_device_signature encoding".into()))?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| AppError::BadRequest("source_device_signature must be 64 bytes".into()))?;

    let row = sqlx::query(
        r#"
        SELECT signing_key
        FROM identity_keys
        WHERE user_id = $1 AND device_id = $2
        "#,
    )
    .bind(user_id)
    .bind(source_signal_device_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::Conflict("Backup source device is missing identity keys".into()))?;

    let signing_key: Vec<u8> = row.try_get("signing_key")?;
    let signing_key_bytes: [u8; 32] = signing_key
        .try_into()
        .map_err(|_| AppError::Conflict("Backup source device signing key is invalid".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&signing_key_bytes)
        .map_err(|_| AppError::Conflict("Backup source device signing key is invalid".into()))?;
    let signature = Signature::from_bytes(&signature_bytes);
    let message = backup_restore_message(current_device_id, source_device_id);
    verifying_key
        .verify(&message, &signature)
        .map_err(|_| AppError::Forbidden)
}
