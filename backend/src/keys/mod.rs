use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
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
        return Err(AppError::BadRequest("dh_public_key must be 32 bytes".into()));
    }
    if sig_key.len() != 32 {
        return Err(AppError::BadRequest("signing_public_key must be 32 bytes".into()));
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
    if signature.len() != 64 {
        return Err(AppError::BadRequest("signature must be 64 bytes".into()));
    }

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

async fn upload_one_time_prekeys(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UploadOtpkReq>,
) -> AppResult<Json<serde_json::Value>> {
    if req.keys.is_empty() || req.keys.len() > 200 {
        return Err(AppError::BadRequest("Provide 1–200 one-time prekeys".into()));
    }

    let mut tx = state.db.pool().begin().await?;
    for item in &req.keys {
        let pub_key = BASE64
            .decode(&item.public_key)
            .map_err(|_| AppError::BadRequest(format!("Bad encoding for key_id {}", item.key_id)))?;
        if pub_key.len() != 32 {
            return Err(AppError::BadRequest(format!(
                "public_key for key_id {} must be 32 bytes",
                item.key_id
            )));
        }
        sqlx::query(
            r#"
            INSERT INTO one_time_prekeys (user_id, device_id, key_id, public_key)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, device_id, key_id) DO NOTHING
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
    Ok(Json(serde_json::json!({ "count": count, "low": count < 10 })))
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

async fn get_key_bundle(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<KeyBundle>> {
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
        return Err(AppError::BadRequest("encrypted_blob exceeds 10 MB limit".into()));
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

async fn get_backup(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<BackupResp>> {
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
}
