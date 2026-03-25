use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, RateLimiter};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::num::NonZeroU32;
use uuid::Uuid;

use crate::{
    auth::{AuthDevice, AuthUser},
    devices::{
        enqueue_sync_event, get_device_for_user, send_live_sync_event, trusted_devices_for_user,
        DeviceTrustState,
    },
    error::{AppError, AppResult},
    users, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/identity", post(upload_identity_key))
        .route("/signed-prekey", post(upload_signed_prekey))
        .route("/one-time-prekeys", post(upload_one_time_prekeys))
        // Static route must come before :user_id to win priority
        .route("/one-time-prekey-count", get(get_opk_count))
        .route("/backup", get(get_backup).put(put_backup))
        .route("/:user_id", get(get_key_bundle))
}

pub fn v2_router() -> Router<AppState> {
    Router::new()
        .route("/identity", post(upload_identity_key_v2))
        .route("/signed-prekey", post(upload_signed_prekey_v2))
        .route("/one-time-prekeys", post(upload_one_time_prekeys_v2))
        .route("/one-time-prekey-count", get(get_opk_count_v2))
        .route("/backup", get(get_backup_v2).put(put_backup_v2))
        .route("/backup/restore", post(restore_backup_v2))
        .route("/:user_id/bundles", get(get_key_bundles_v2))
}

type KeyedLimiter<T> = RateLimiter<T, DefaultKeyedStateStore<T>, DefaultClock>;

static BACKUP_RETRIEVE_LIMITER: once_cell::sync::Lazy<KeyedLimiter<Uuid>> =
    once_cell::sync::Lazy::new(|| {
        RateLimiter::keyed(
            governor::Quota::with_period(std::time::Duration::from_secs(60 * 60))
                .expect("valid backup quota")
                .allow_burst(NonZeroU32::new(5).expect("non-zero burst")),
        )
    });

/// Reject all-zero X25519 keys, which lie in a small subgroup and would
/// produce predictable shared secrets.
fn reject_trivial_x25519_key(bytes: &[u8], field: &str) -> AppResult<()> {
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
async fn verify_signed_prekey_signature(
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

// ─── Upload Identity Key ─────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadIdentityKeyReq {
    device_id: i32,
    /// Base64-encoded X25519 public key (32 bytes) — used for DH in X3DH.
    dh_public_key: String,
    /// Base64-encoded Ed25519 public key (32 bytes) — used to verify signed prekeys.
    signing_public_key: String,
}

/// POST /api/v1/keys/identity — upload or rotate an X25519 + Ed25519 identity key pair.
///
/// # E2EE contract
///
/// * `dh_public_key` (X25519) is used in X3DH key agreement.
/// * `signing_public_key` (Ed25519) is used to verify signed prekeys.
/// * Both are public-only; the server never receives private keys.
async fn upload_identity_key(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UploadIdentityKeyReq>,
) -> AppResult<Json<serde_json::Value>> {
    let dh_key = BASE64
        .decode(&req.dh_public_key)
        .map_err(|_| AppError::BadRequest("Invalid dh_public_key encoding".into()))?;
    let sig_key = BASE64
        .decode(&req.signing_public_key)
        .map_err(|_| AppError::BadRequest("Invalid signing_public_key encoding".into()))?;

    if dh_key.len() != 32 {
        return Err(AppError::BadRequest(
            "dh_public_key must be 32 bytes".into(),
        ));
    }
    reject_trivial_x25519_key(&dh_key, "dh_public_key")?;
    if sig_key.len() != 32 {
        return Err(AppError::BadRequest(
            "signing_public_key must be 32 bytes".into(),
        ));
    }

    sqlx::query(
        r#"
        INSERT INTO identity_keys (user_id, device_id, public_key, signing_key)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, device_id) DO UPDATE
            SET public_key  = EXCLUDED.public_key,
                signing_key = EXCLUDED.signing_key,
                created_at  = NOW()
        "#,
    )
    .bind(auth.user_id)
    .bind(req.device_id)
    .bind(&dh_key)
    .bind(&sig_key)
    .execute(state.db.pool())
    .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

// ─── Upload Signed PreKey ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadSignedPreKeyReq {
    device_id: i32,
    key_id: i32,
    /// Base64-encoded X25519 public key (32 bytes).
    public_key: String,
    /// Base64-encoded Ed25519 signature over public_key (64 bytes).
    signature: String,
}

/// POST /api/v1/keys/signed-prekey — upload a signed prekey (rotates weekly).
///
/// The Ed25519 signature over the X25519 public key is verified server-side
/// against the device's stored signing key before persisting.
async fn upload_signed_prekey(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UploadSignedPreKeyReq>,
) -> AppResult<Json<serde_json::Value>> {
    let public_key = BASE64
        .decode(&req.public_key)
        .map_err(|_| AppError::BadRequest("Invalid public_key encoding".into()))?;
    let signature = BASE64
        .decode(&req.signature)
        .map_err(|_| AppError::BadRequest("Invalid signature encoding".into()))?;

    if public_key.len() != 32 {
        return Err(AppError::BadRequest("public_key must be 32 bytes".into()));
    }
    reject_trivial_x25519_key(&public_key, "public_key")?;
    if signature.len() != 64 {
        return Err(AppError::BadRequest("signature must be 64 bytes".into()));
    }

    verify_signed_prekey_signature(&state, auth.user_id, req.device_id, &public_key, &signature)
        .await?;

    // Signed prekeys rotate weekly.
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    sqlx::query(
        r#"
        INSERT INTO signed_prekeys (user_id, device_id, key_id, public_key, signature, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (user_id, device_id, key_id) DO UPDATE
            SET public_key  = EXCLUDED.public_key,
                signature   = EXCLUDED.signature,
                expires_at  = EXCLUDED.expires_at,
                created_at  = NOW()
        "#,
    )
    .bind(auth.user_id)
    .bind(req.device_id)
    .bind(req.key_id)
    .bind(&public_key)
    .bind(&signature)
    .bind(expires_at)
    .execute(state.db.pool())
    .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

// ─── Upload One-Time PreKeys ──────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OtpkItem {
    key_id: i32,
    /// Base64-encoded X25519 public key (32 bytes).
    public_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadOtpkReq {
    device_id: i32,
    keys: Vec<OtpkItem>,
}

/// POST /api/v1/keys/one-time-prekeys — upload a batch of one-time prekeys.
///
/// Batch size is capped at [`MAX_OPK_BATCH`](crate::constants::MAX_OPK_BATCH).
/// Each key is validated (32 bytes, non-trivial) and upserted in a single transaction.
///
/// # E2EE contract
///
/// * OPKs are consumed atomically via `FOR UPDATE SKIP LOCKED` during X3DH.
///   Each OPK is used at most once, providing forward secrecy for the initial message.
async fn upload_one_time_prekeys(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UploadOtpkReq>,
) -> AppResult<Json<serde_json::Value>> {
    if req.keys.is_empty() || req.keys.len() > crate::constants::MAX_OPK_BATCH {
        return Err(AppError::BadRequest(format!(
            "Provide 1–{} one-time prekeys",
            crate::constants::MAX_OPK_BATCH
        )));
    }

    let mut tx = state.db.pool().begin().await?;
    for item in &req.keys {
        let pub_key = BASE64.decode(&item.public_key).map_err(|_| {
            AppError::BadRequest(format!("Bad encoding for key_id {}", item.key_id))
        })?;
        if pub_key.len() != 32 {
            return Err(AppError::BadRequest(format!(
                "public_key for key_id {} must be 32 bytes",
                item.key_id
            )));
        }
        reject_trivial_x25519_key(&pub_key, &format!("public_key for key_id {}", item.key_id))?;
        sqlx::query(
            r#"
            INSERT INTO one_time_prekeys (user_id, device_id, key_id, public_key)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, device_id, key_id) DO UPDATE
                SET public_key = EXCLUDED.public_key, consumed = FALSE
            "#,
        )
        .bind(auth.user_id)
        .bind(req.device_id)
        .bind(item.key_id)
        .bind(&pub_key)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "uploaded": req.keys.len() })))
}

// ─── Get OPK Count ────────────────────────────────────────────────────────────

/// GET /api/v1/keys/one-time-prekey-count — returns the number of unconsumed OPKs
/// and a `low` flag (true when < 10 remain) so clients know when to replenish.
async fn get_opk_count(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) AS count
        FROM one_time_prekeys
        WHERE user_id = $1 AND device_id = 1 AND consumed = FALSE
        "#,
    )
    .bind(auth.user_id)
    .fetch_one(state.db.pool())
    .await?;

    let count: i64 = row.try_get("count")?;
    Ok(Json(
        serde_json::json!({ "count": count, "low": count < 10 }),
    ))
}

// ─── Get Key Bundle ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct KeyBundle {
    user_id: Uuid,
    device_id: i32,
    /// Base64-encoded X25519 public key.
    identity_dh_key: String,
    /// Base64-encoded Ed25519 public key.
    identity_sig_key: String,
    signed_prekey_id: i32,
    /// Base64-encoded X25519 public key.
    signed_prekey: String,
    /// Base64-encoded Ed25519 signature.
    signed_prekey_sig: String,
    one_time_prekey_id: Option<i32>,
    /// Base64-encoded X25519 public key (absent if OPKs exhausted).
    one_time_prekey: Option<String>,
}

/// GET /api/v1/keys/:user_id — fetch a user's public key bundle for X3DH.
///
/// Returns the identity key pair (DH + signing), latest signed prekey, and
/// atomically consumes one OPK (if available) via `FOR UPDATE SKIP LOCKED`.
///
/// # Security invariants
///
/// * Access control: caller must share a DM conversation or server membership
///   with the target user ([`require_key_bundle_access`]).
/// * OPK consumption is atomic — concurrent requests never consume the same OPK.
///
/// # E2EE contract
///
/// * Only public keys are returned. The server never has access to private keys.
async fn get_key_bundle(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<KeyBundle>> {
    users::require_key_bundle_access(auth.user_id, user_id, &state).await?;

    // Identity key
    let identity_row = sqlx::query(
        r#"
        SELECT device_id, public_key, signing_key
        FROM identity_keys
        WHERE user_id = $1 AND device_id = 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("User has no registered keys".into()))?;

    let device_id: i32 = identity_row.try_get("device_id")?;
    let dh_bytes: Vec<u8> = identity_row.try_get("public_key")?;
    let sig_bytes: Option<Vec<u8>> = identity_row.try_get("signing_key")?;
    let sig_bytes =
        sig_bytes.ok_or_else(|| AppError::NotFound("User has no signing key".into()))?;

    // Latest non-expired signed prekey
    let spk_row = sqlx::query(
        r#"
        SELECT key_id, public_key, signature
        FROM signed_prekeys
        WHERE user_id = $1 AND device_id = $2 AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(device_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("User has no valid signed prekey".into()))?;

    let spk_id: i32 = spk_row.try_get("key_id")?;
    let spk_pub: Vec<u8> = spk_row.try_get("public_key")?;
    let spk_sig: Vec<u8> = spk_row.try_get("signature")?;

    // Atomically consume one OPK (SKIP LOCKED avoids contention on concurrent fetches)
    let opk_row = sqlx::query(
        r#"
        UPDATE one_time_prekeys
        SET consumed = TRUE
        WHERE id = (
            SELECT id FROM one_time_prekeys
            WHERE user_id = $1 AND device_id = $2 AND consumed = FALSE
            ORDER BY id
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING key_id, public_key
        "#,
    )
    .bind(user_id)
    .bind(device_id)
    .fetch_optional(state.db.pool())
    .await?;

    let (opk_id, opk_pub) = match opk_row {
        Some(row) => {
            let id: i32 = row.try_get("key_id")?;
            let pub_key: Vec<u8> = row.try_get("public_key")?;
            (Some(id), Some(BASE64.encode(&pub_key)))
        }
        None => (None, None),
    };

    Ok(Json(KeyBundle {
        user_id,
        device_id,
        identity_dh_key: BASE64.encode(&dh_bytes),
        identity_sig_key: BASE64.encode(&sig_bytes),
        signed_prekey_id: spk_id,
        signed_prekey: BASE64.encode(&spk_pub),
        signed_prekey_sig: BASE64.encode(&spk_sig),
        one_time_prekey_id: opk_id,
        one_time_prekey: opk_pub,
    }))
}

// ─── PIN Key Backup ───────────────────────────────────────────────────────────
// The server stores an opaque encrypted blob; it never sees the PIN or plaintext.
// Client encrypts: PBKDF2(PIN, salt, 600_000 iters) → AES-256-GCM key.
// Blob format (client-defined): base64( salt || iv || ciphertext || tag ).

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PutBackupReq {
    /// Base64-encoded opaque encrypted blob.
    encrypted_blob: String,
}

#[derive(Serialize)]
struct BackupResp {
    /// Base64-encoded opaque encrypted blob.
    encrypted_blob: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct BackupRespV2 {
    encrypted_blob: String,
    version: i32,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupQueryV2 {
    source_device_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreBackupReqV2 {
    source_device_id: Uuid,
    source_device_signature: String,
    #[serde(default)]
    replace_source_device: bool,
}

#[derive(Serialize)]
struct KeyBundleV2 {
    user_id: Uuid,
    device_id: Uuid,
    signal_device_id: i32,
    identity_dh_key: String,
    identity_sig_key: String,
    signed_prekey_id: i32,
    signed_prekey: String,
    signed_prekey_sig: String,
    one_time_prekey_id: Option<i32>,
    one_time_prekey: Option<String>,
}

/// PUT /api/v1/keys/backup — store an opaque PIN-encrypted key backup blob.
///
/// The server stores the blob as-is; it never sees the PIN or plaintext.
/// Client encrypts: `PBKDF2(PIN, salt, 1_200_000) → AES-256-GCM key`.
///
/// # Security invariants
///
/// * Maximum blob size: 10 MB. Minimum: 45 bytes (salt + IV + 1 byte + tag).
/// * Server cannot decrypt — zero-knowledge backup.
async fn put_backup(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<PutBackupReq>,
) -> AppResult<Json<serde_json::Value>> {
    let blob = BASE64
        .decode(&req.encrypted_blob)
        .map_err(|_| AppError::BadRequest("Invalid encrypted_blob encoding".into()))?;

    // Sanity: salt(16) + iv(12) + at least 1 byte + tag(16) = 45 bytes minimum
    if blob.len() < 45 {
        return Err(AppError::BadRequest("encrypted_blob too short".into()));
    }
    // 10 MB cap — keystore should be kilobytes even with many sessions
    if blob.len() > 10 * 1024 * 1024 {
        return Err(AppError::BadRequest(
            "encrypted_blob exceeds 10 MB limit".into(),
        ));
    }

    sqlx::query(
        r#"
        INSERT INTO key_backups (user_id, encrypted_blob, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_id) DO UPDATE
            SET encrypted_blob = EXCLUDED.encrypted_blob,
                updated_at     = NOW()
        "#,
    )
    .bind(auth.user_id)
    .bind(&blob)
    .execute(state.db.pool())
    .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// GET /api/v1/keys/backup — retrieve the PIN-encrypted key backup blob.
///
/// Rate-limited to 5 retrievals per hour per user to prevent offline PIN brute-force.
async fn get_backup(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<BackupResp>> {
    // H-06: Rate limit backup retrieval to prevent PIN brute-force (5 attempts/hour/user)
    if BACKUP_RETRIEVE_LIMITER.check_key(&auth.user_id).is_err() {
        return Err(AppError::RateLimited);
    }

    let row = sqlx::query(
        r#"
        SELECT encrypted_blob, updated_at
        FROM key_backups
        WHERE user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("No backup found".into()))?;

    let blob: Vec<u8> = row.try_get("encrypted_blob")?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;

    Ok(Json(BackupResp {
        encrypted_blob: BASE64.encode(&blob),
        updated_at,
    }))
}

/// POST /api/v2/keys/identity — v2 identity key upload (device-aware, trusted-only).
async fn upload_identity_key_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(mut req): Json<UploadIdentityKeyReq>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_trusted()?;
    req.device_id = auth.signal_device_id;
    upload_identity_key(
        AuthUser {
            user_id: auth.user_id,
            device_id: Some(auth.device_id),
            account_type: auth.account_type,
        },
        State(state),
        Json(req),
    )
    .await
}

/// POST /api/v2/keys/signed-prekey — v2 signed prekey upload (device-aware, trusted-only).
async fn upload_signed_prekey_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(mut req): Json<UploadSignedPreKeyReq>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_trusted()?;
    req.device_id = auth.signal_device_id;
    upload_signed_prekey(
        AuthUser {
            user_id: auth.user_id,
            device_id: Some(auth.device_id),
            account_type: auth.account_type,
        },
        State(state),
        Json(req),
    )
    .await
}

/// POST /api/v2/keys/one-time-prekeys — v2 OPK batch upload (device-aware, trusted-only).
async fn upload_one_time_prekeys_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(mut req): Json<UploadOtpkReq>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_trusted()?;
    req.device_id = auth.signal_device_id;
    upload_one_time_prekeys(
        AuthUser {
            user_id: auth.user_id,
            device_id: Some(auth.device_id),
            account_type: auth.account_type,
        },
        State(state),
        Json(req),
    )
    .await
}

/// GET /api/v2/keys/one-time-prekey-count — v2 OPK count (device-scoped).
async fn get_opk_count_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_trusted()?;
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) AS count
        FROM one_time_prekeys
        WHERE user_id = $1 AND device_id = $2 AND consumed = FALSE
        "#,
    )
    .bind(auth.user_id)
    .bind(auth.signal_device_id)
    .fetch_one(state.db.pool())
    .await?;

    let count: i64 = row.try_get("count")?;
    Ok(Json(
        serde_json::json!({ "count": count, "low": count < 10 }),
    ))
}

/// GET /api/v2/keys/:user_id/bundles — fetch key bundles for all (or filtered) devices.
///
/// Returns one bundle per non-revoked device. Optionally consumes an OPK per
/// device when `consume_opk=true`. Device IDs can be filtered via `device_ids`
/// query param (comma-separated, max [`MAX_DEVICE_IDS`](crate::constants::MAX_DEVICE_IDS)).
///
/// # Security invariants
///
/// * Requires a trusted device.
/// * Access control via [`require_key_bundle_access`] (DM or shared server).
/// * Device ID count capped to prevent query amplification.
async fn get_key_bundles_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<GetKeyBundlesQuery>,
) -> AppResult<Json<Vec<KeyBundleV2>>> {
    auth.require_trusted()?;
    users::require_key_bundle_access(auth.user_id, user_id, &state).await?;
    let consume_opk = query.consume_opk.unwrap_or(false);
    let device_ids = parse_device_ids_filter(query.device_ids.as_deref())?;

    // Step 1: Fetch device + identity + signed-prekey rows (no OPK consumption yet).
    // LATERAL (UPDATE ...) is not valid PostgreSQL syntax, so OPK consumption is
    // handled separately below via a CTE-based UPDATE.
    let rows = match device_ids.as_ref() {
        Some(device_ids) => {
            sqlx::query(
                r#"
                SELECT d.id AS device_id,
                       d.signal_device_id,
                       ik.public_key,
                       ik.signing_key,
                       sp.key_id AS signed_prekey_id,
                       sp.public_key AS signed_prekey,
                       sp.signature AS signed_prekey_sig
                FROM devices d
                JOIN identity_keys ik
                  ON ik.user_id = d.user_id
                 AND ik.device_id = d.signal_device_id
                JOIN LATERAL (
                    SELECT key_id, public_key, signature
                    FROM signed_prekeys
                    WHERE user_id = d.user_id
                      AND device_id = d.signal_device_id
                      AND expires_at > NOW()
                    ORDER BY created_at DESC
                    LIMIT 1
                ) sp ON TRUE
                WHERE d.user_id = $1
                  AND d.revoked_at IS NULL
                  AND d.trust_state = 'trusted'
                  AND d.id = ANY($2)
                ORDER BY d.created_at ASC
                "#,
            )
            .bind(user_id)
            .bind(device_ids)
            .fetch_all(state.db.pool())
            .await?
        }
        None => {
            sqlx::query(
                r#"
                SELECT d.id AS device_id,
                       d.signal_device_id,
                       ik.public_key,
                       ik.signing_key,
                       sp.key_id AS signed_prekey_id,
                       sp.public_key AS signed_prekey,
                       sp.signature AS signed_prekey_sig
                FROM devices d
                JOIN identity_keys ik
                  ON ik.user_id = d.user_id
                 AND ik.device_id = d.signal_device_id
                JOIN LATERAL (
                    SELECT key_id, public_key, signature
                    FROM signed_prekeys
                    WHERE user_id = d.user_id
                      AND device_id = d.signal_device_id
                      AND expires_at > NOW()
                    ORDER BY created_at DESC
                    LIMIT 1
                ) sp ON TRUE
                WHERE d.user_id = $1
                  AND d.revoked_at IS NULL
                  AND d.trust_state = 'trusted'
                ORDER BY d.created_at ASC
                "#,
            )
            .bind(user_id)
            .fetch_all(state.db.pool())
            .await?
        }
    };

    if rows.is_empty() {
        return Err(AppError::NotFound(
            "User has no trusted device bundles".into(),
        ));
    }

    // Step 2: Build bundles; atomically consume one OPK per device when requested.
    // Uses a writable CTE so the FOR UPDATE SKIP LOCKED lock is in a top-level SELECT
    // (valid) rather than in a subquery of a DML statement (invalid in PostgreSQL).
    let mut bundles = Vec::with_capacity(rows.len());
    for row in rows {
        let signal_device_id: i32 = row.try_get("signal_device_id")?;

        let (opk_id, opk_pub) = if consume_opk {
            let opk_row = sqlx::query(
                r#"
                WITH cte AS (
                    SELECT id, key_id, public_key
                    FROM one_time_prekeys
                    WHERE user_id = $1
                      AND device_id = $2
                      AND consumed = FALSE
                    ORDER BY id
                    LIMIT 1
                    FOR UPDATE SKIP LOCKED
                )
                UPDATE one_time_prekeys
                SET consumed = TRUE
                FROM cte
                WHERE one_time_prekeys.id = cte.id
                RETURNING one_time_prekeys.key_id, one_time_prekeys.public_key
                "#,
            )
            .bind(user_id)
            .bind(signal_device_id)
            .fetch_optional(state.db.pool())
            .await?;

            match opk_row {
                Some(r) => {
                    let key_id: Option<i32> = r.try_get("key_id").ok();
                    let pub_key: Option<Vec<u8>> = r.try_get("public_key").ok();
                    (key_id, pub_key)
                }
                None => (None, None),
            }
        } else {
            (None, None)
        };

        let dh_bytes: Vec<u8> = row.try_get("public_key")?;
        let sig_bytes: Vec<u8> = row.try_get("signing_key")?;
        let spk_pub: Vec<u8> = row.try_get("signed_prekey")?;
        let spk_sig: Vec<u8> = row.try_get("signed_prekey_sig")?;
        bundles.push(KeyBundleV2 {
            user_id,
            device_id: row.try_get("device_id")?,
            signal_device_id: row.try_get("signal_device_id")?,
            identity_dh_key: BASE64.encode(&dh_bytes),
            identity_sig_key: BASE64.encode(&sig_bytes),
            signed_prekey_id: row.try_get("signed_prekey_id")?,
            signed_prekey: BASE64.encode(&spk_pub),
            signed_prekey_sig: BASE64.encode(&spk_sig),
            one_time_prekey_id: opk_id,
            one_time_prekey: opk_pub.map(|bytes| BASE64.encode(&bytes)),
        });
    }

    Ok(Json(bundles))
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct GetKeyBundlesQuery {
    consume_opk: Option<bool>,
    device_ids: Option<String>,
}

fn parse_device_ids_filter(value: Option<&str>) -> AppResult<Option<Vec<Uuid>>> {
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

fn backup_restore_message(current_device_id: Uuid, source_device_id: Uuid) -> Vec<u8> {
    format!("yapper-backup-restore:v1:{current_device_id}:{source_device_id}").into_bytes()
}

async fn verify_backup_restore_signature(
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

async fn put_backup_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(req): Json<PutBackupReq>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_trusted()?;
    let blob = BASE64
        .decode(&req.encrypted_blob)
        .map_err(|_| AppError::BadRequest("Invalid encrypted_blob encoding".into()))?;

    if blob.len() < 45 {
        return Err(AppError::BadRequest("encrypted_blob too short".into()));
    }
    if blob.len() > 10 * 1024 * 1024 {
        return Err(AppError::BadRequest(
            "encrypted_blob exceeds 10 MB limit".into(),
        ));
    }

    let version_row = sqlx::query(
        r#"
        SELECT COALESCE(MAX(version), 0) + 1 AS next_version
        FROM device_key_backups
        WHERE user_id = $1 AND device_id = $2
        "#,
    )
    .bind(auth.user_id)
    .bind(auth.device_id)
    .fetch_one(state.db.pool())
    .await?;
    let next_version: i32 = version_row.try_get("next_version")?;

    sqlx::query(
        r#"
        INSERT INTO device_key_backups
            (user_id, device_id, source_device_id, encrypted_blob, version, updated_at)
        VALUES ($1, $2, $2, $3, $4, NOW())
        "#,
    )
    .bind(auth.user_id)
    .bind(auth.device_id)
    .bind(&blob)
    .bind(next_version)
    .execute(state.db.pool())
    .await?;

    Ok(Json(
        serde_json::json!({ "status": "ok", "version": next_version }),
    ))
}

async fn get_backup_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Query(query): Query<BackupQueryV2>,
) -> AppResult<Json<BackupRespV2>> {
    if BACKUP_RETRIEVE_LIMITER.check_key(&auth.device_id).is_err() {
        return Err(AppError::RateLimited);
    }

    let source_device_id = query.source_device_id.unwrap_or(auth.device_id);
    let source_device = get_device_for_user(auth.user_id, source_device_id, &state).await?;

    if source_device.revoked_at.is_some() {
        return Err(AppError::Conflict("Backup source device is revoked".into()));
    }

    if auth.device_id != source_device_id {
        if source_device.trust_state != DeviceTrustState::Trusted {
            return Err(AppError::Conflict(
                "Backup source device is not trusted".into(),
            ));
        }
    } else {
        auth.require_trusted()?;
    }

    let row = sqlx::query(
        r#"
        SELECT encrypted_blob, version, updated_at
        FROM device_key_backups
        WHERE user_id = $1 AND device_id = $2 AND revoked_at IS NULL
        ORDER BY version DESC
        LIMIT 1
        "#,
    )
    .bind(auth.user_id)
    .bind(source_device_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("No backup found".into()))?;

    let blob: Vec<u8> = row.try_get("encrypted_blob")?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    let version: i32 = row.try_get("version")?;

    Ok(Json(BackupRespV2 {
        encrypted_blob: BASE64.encode(&blob),
        version,
        updated_at,
    }))
}

async fn restore_backup_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(req): Json<RestoreBackupReqV2>,
) -> AppResult<Json<serde_json::Value>> {
    if auth.device_id == req.source_device_id {
        return Err(AppError::BadRequest(
            "source_device_id must differ from the current device".into(),
        ));
    }

    let current_device = get_device_for_user(auth.user_id, auth.device_id, &state).await?;
    if current_device.revoked_at.is_some() {
        return Err(AppError::Conflict("Current device is revoked".into()));
    }

    let source_device = get_device_for_user(auth.user_id, req.source_device_id, &state).await?;
    if source_device.revoked_at.is_some() {
        return Err(AppError::Conflict("Backup source device is revoked".into()));
    }
    if source_device.trust_state != DeviceTrustState::Trusted {
        return Err(AppError::Conflict(
            "Backup source device is not trusted".into(),
        ));
    }
    verify_backup_restore_signature(
        &state,
        auth.user_id,
        auth.device_id,
        req.source_device_id,
        source_device.signal_device_id,
        &req.source_device_signature,
    )
    .await?;

    let backup_exists = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM device_key_backups
        WHERE user_id = $1 AND device_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(auth.user_id)
    .bind(req.source_device_id)
    .fetch_one(state.db.pool())
    .await?;
    if backup_exists == 0 {
        return Err(AppError::NotFound("No backup found".into()));
    }

    let mut tx = state.db.pool().begin().await?;

    sqlx::query(
        r#"
        UPDATE devices
        SET trust_state = 'trusted',
            approved_at = COALESCE(approved_at, NOW()),
            approved_by = $1,
            last_seen_at = COALESCE(last_seen_at, NOW())
        WHERE id = $2 AND user_id = $3
        "#,
    )
    .bind(req.source_device_id)
    .bind(auth.device_id)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE device_trust_requests
        SET approved_at = COALESCE(approved_at, NOW()),
            approved_by = COALESCE(approved_by, $1)
        WHERE target_device_id = $2
        "#,
    )
    .bind(req.source_device_id)
    .bind(auth.device_id)
    .execute(&mut *tx)
    .await?;

    if req.replace_source_device {
        sqlx::query(
            r#"
            UPDATE devices
            SET trust_state = 'revoked',
                revoked_at = NOW()
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(req.source_device_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE sessions SET revoked_at = NOW() WHERE device_id = $1 AND revoked_at IS NULL",
        )
        .bind(req.source_device_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE device_key_backups
            SET revoked_at = NOW()
            WHERE device_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(req.source_device_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    if req.replace_source_device {
        send_live_sync_event(
            req.source_device_id,
            Some(auth.device_id),
            "device_revoked",
            serde_json::json!({ "device_id": req.source_device_id }),
            &state,
        );

        for device in trusted_devices_for_user(auth.user_id, &state).await? {
            if device.id == auth.device_id || device.id == req.source_device_id {
                continue;
            }
            enqueue_sync_event(
                auth.user_id,
                device.id,
                Some(auth.device_id),
                "device_revoked",
                serde_json::json!({ "device_id": req.source_device_id }),
                &state,
            )
            .await?;
        }
    } else {
        for device in trusted_devices_for_user(auth.user_id, &state).await? {
            if device.id == auth.device_id {
                continue;
            }
            enqueue_sync_event(
                auth.user_id,
                device.id,
                Some(req.source_device_id),
                "device_trust_approved",
                serde_json::json!({
                    "target_device_id": auth.device_id,
                    "approved_by": req.source_device_id,
                    "via_backup_restore": true,
                }),
                &state,
            )
            .await?;
        }
    }

    Ok(Json(serde_json::json!({
        "status": "restored",
        "device_id": auth.device_id,
        "source_device_id": req.source_device_id,
        "source_device_replaced": req.replace_source_device,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let bytes = [0u8; 32];
        let encoded = BASE64.encode(bytes);
        let decoded = BASE64.decode(&encoded).unwrap();
        assert_eq!(bytes.to_vec(), decoded);
    }

    #[test]
    fn parse_device_ids_filter_accepts_uuid_lists() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let raw = format!("{first}, {second}");

        let parsed = parse_device_ids_filter(Some(&raw)).unwrap().unwrap();
        assert_eq!(parsed, vec![first, second]);
    }

    #[test]
    fn parse_device_ids_filter_rejects_invalid_uuid() {
        let err = parse_device_ids_filter(Some("not-a-uuid")).unwrap_err();
        match err {
            AppError::BadRequest(message) => {
                assert!(message.contains("Invalid device_id"));
            }
            other => panic!("expected bad request, got {other:?}"),
        }
    }

    #[test]
    fn restore_backup_request_defaults_to_non_destructive_mode() {
        let source_device_id = Uuid::new_v4();
        let req: RestoreBackupReqV2 = serde_json::from_value(serde_json::json!({
            "source_device_id": source_device_id,
            "source_device_signature": BASE64.encode([9u8; 64]),
        }))
        .expect("request should deserialize");

        assert_eq!(req.source_device_id, source_device_id);
        assert!(!req.source_device_signature.is_empty());
        assert!(!req.replace_source_device);
    }

    #[test]
    fn restore_backup_request_allows_explicit_source_replacement() {
        let req: RestoreBackupReqV2 = serde_json::from_value(serde_json::json!({
            "source_device_id": Uuid::new_v4(),
            "source_device_signature": BASE64.encode([9u8; 64]),
            "replace_source_device": true,
        }))
        .expect("request should deserialize");

        assert!(req.replace_source_device);
    }
}
