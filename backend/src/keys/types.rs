use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadIdentityKeyReq {
    pub device_id: i32,
    /// Base64-encoded X25519 public key (32 bytes) — used for DH in X3DH.
    pub dh_public_key: String,
    /// Base64-encoded Ed25519 public key (32 bytes) — used to verify signed prekeys.
    pub signing_public_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadSignedPreKeyReq {
    pub device_id: i32,
    pub key_id: i32,
    /// Base64-encoded X25519 public key (32 bytes).
    pub public_key: String,
    /// Base64-encoded Ed25519 signature over public_key (64 bytes).
    pub signature: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtpkItem {
    pub key_id: i32,
    /// Base64-encoded X25519 public key (32 bytes).
    pub public_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadOtpkReq {
    pub device_id: i32,
    pub keys: Vec<OtpkItem>,
}

#[derive(Serialize)]
pub struct KeyBundle {
    pub user_id: Uuid,
    pub device_id: i32,
    /// Base64-encoded X25519 public key.
    pub identity_dh_key: String,
    /// Base64-encoded Ed25519 public key.
    pub identity_sig_key: String,
    pub signed_prekey_id: i32,
    /// Base64-encoded X25519 public key.
    pub signed_prekey: String,
    /// Base64-encoded Ed25519 signature.
    pub signed_prekey_sig: String,
    pub one_time_prekey_id: Option<i32>,
    /// Base64-encoded X25519 public key (absent if OPKs exhausted).
    pub one_time_prekey: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutBackupReq {
    /// Base64-encoded opaque encrypted blob.
    pub encrypted_blob: String,
}

#[derive(Serialize)]
pub struct BackupResp {
    /// Base64-encoded opaque encrypted blob.
    pub encrypted_blob: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct BackupRespV2 {
    pub encrypted_blob: String,
    pub version: i32,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupQueryV2 {
    pub source_device_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreBackupReqV2 {
    pub source_device_id: Uuid,
    pub source_device_signature: String,
    #[serde(default)]
    pub replace_source_device: bool,
}

#[derive(Serialize)]
pub struct KeyBundleV2 {
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub signal_device_id: i32,
    pub identity_dh_key: String,
    pub identity_sig_key: String,
    pub signed_prekey_id: i32,
    pub signed_prekey: String,
    pub signed_prekey_sig: String,
    pub one_time_prekey_id: Option<i32>,
    pub one_time_prekey: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct GetKeyBundlesQuery {
    pub consume_opk: Option<bool>,
    pub device_ids: Option<String>,
}
