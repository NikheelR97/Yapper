pub mod handlers;
pub mod service;
pub mod types;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;
use handlers::*;

pub fn router() -> Router<AppState> {
    Router::new()
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use uuid::Uuid;

    use crate::error::AppError;
    use service::parse_device_ids_filter;
    use types::RestoreBackupReqV2;

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
