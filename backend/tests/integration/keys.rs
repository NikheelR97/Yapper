//! Integration tests — key server endpoints.
//!
//! Verifies identity key upload, OPK bundle management, and atomic OPK consumption.

use super::{
    authorization_header_name, bearer_header, create_test_user, csrf_header_name,
    csrf_header_value, spawn_test_server,
};
use serial_test::serial;
use uuid::Uuid;

/// Generate a fake X25519 key (32 bytes) encoded as base64 for test payloads.
fn fake_key_b64() -> String {
    use base64::Engine;
    let bytes: Vec<u8> = (0..32).map(|i| i as u8).collect();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// Upload identity + signed prekey + OPKs, then verify key bundle is fetchable.
#[tokio::test]
#[serial]
async fn keys_upload_and_fetch_bundle() {
    let Some(server) = spawn_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_user_id, access_token, csrf_token) = create_test_user(&server, &suffix).await;

    // Upload identity key
    let ik_upload = server
        .post("/api/v1/keys/identity")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
        .json(&serde_json::json!({
            "identity_key": fake_key_b64(),
            "signing_key": fake_key_b64(),
        }))
        .await;
    assert!(
        ik_upload.status_code().is_success(),
        "identity key upload failed: {} — {}",
        ik_upload.status_code(),
        ik_upload.text()
    );

    // Upload signed prekey
    let spk_upload = server
        .post("/api/v1/keys/signed-prekey")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
        .json(&serde_json::json!({
            "key_id": 1,
            "public_key": fake_key_b64(),
            "signature": fake_key_b64(),
        }))
        .await;
    assert!(
        spk_upload.status_code().is_success(),
        "SPK upload failed: {} — {}",
        spk_upload.status_code(),
        spk_upload.text()
    );

    // Upload a batch of OPKs
    let opks: Vec<serde_json::Value> = (1..=5)
        .map(|id| serde_json::json!({ "key_id": id, "public_key": fake_key_b64() }))
        .collect();
    let opk_upload = server
        .post("/api/v1/keys/one-time-prekeys")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
        .json(&serde_json::json!({ "keys": opks }))
        .await;
    assert!(
        opk_upload.status_code().is_success(),
        "OPK upload failed: {} — {}",
        opk_upload.status_code(),
        opk_upload.text()
    );

    // Fetch the key bundle as another user
    let suffix2 = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, _, _) = create_test_user(&server, &suffix).await;
    let (_u2_id, at2, ct2) = create_test_user(&server, &suffix2).await;

    let bundle = server
        .get(&format!("/api/v1/keys/bundle/{user_id}"))
        .add_header(authorization_header_name(), bearer_header(&at2))
        .add_header(csrf_header_name(), csrf_header_value(&ct2))
        .await;
    assert!(
        bundle.status_code().is_success(),
        "key bundle fetch failed: {} — {}",
        bundle.status_code(),
        bundle.text()
    );

    let body: serde_json::Value = bundle.json();
    assert!(
        body["identity_key"].is_string(),
        "bundle missing identity_key"
    );
    assert!(
        body["signed_prekey"].is_object(),
        "bundle missing signed_prekey"
    );
    // OPK is optional (may be null if exhausted), but should be present after upload
    assert!(
        body.get("one_time_prekey").is_some(),
        "bundle missing one_time_prekey field"
    );
}

/// After OPK consumption, the OPK count should have decremented.
#[tokio::test]
#[serial]
async fn keys_opk_count_decrements_after_consumption() {
    let Some(server) = spawn_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, at, ct) = create_test_user(&server, &suffix).await;

    // Upload identity + SPK (required before OPKs)
    server
        .post("/api/v1/keys/identity")
        .add_header(authorization_header_name(), bearer_header(&at))
        .add_header(csrf_header_name(), csrf_header_value(&ct))
        .json(&serde_json::json!({
            "identity_key": fake_key_b64(),
            "signing_key": fake_key_b64(),
        }))
        .await;
    server
        .post("/api/v1/keys/signed-prekey")
        .add_header(authorization_header_name(), bearer_header(&at))
        .add_header(csrf_header_name(), csrf_header_value(&ct))
        .json(&serde_json::json!({
            "key_id": 1,
            "public_key": fake_key_b64(),
            "signature": fake_key_b64(),
        }))
        .await;

    // Upload exactly 3 OPKs
    let opks: Vec<serde_json::Value> = (100..=102)
        .map(|id| serde_json::json!({ "key_id": id, "public_key": fake_key_b64() }))
        .collect();
    server
        .post("/api/v1/keys/one-time-prekeys")
        .add_header(authorization_header_name(), bearer_header(&at))
        .add_header(csrf_header_name(), csrf_header_value(&ct))
        .json(&serde_json::json!({ "keys": opks }))
        .await;

    // Check initial OPK count
    let count_before = opk_count(&server, &at, &ct).await;

    // Consume one OPK by fetching bundle as another user
    let suffix2 = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_u2, at2, ct2) = create_test_user(&server, &suffix2).await;
    let bundle = server
        .get(&format!("/api/v1/keys/bundle/{user_id}"))
        .add_header(authorization_header_name(), bearer_header(&at2))
        .add_header(csrf_header_name(), csrf_header_value(&ct2))
        .await;
    let bundle_body: serde_json::Value = bundle.json();

    // If an OPK was returned, the count should have decreased
    if bundle_body["one_time_prekey"].is_object() {
        let count_after = opk_count(&server, &at, &ct).await;
        assert!(
            count_after < count_before,
            "OPK count did not decrease after consumption: before={count_before}, after={count_after}"
        );
    }
}

async fn opk_count(server: &axum_test::TestServer, at: &str, ct: &str) -> u64 {
    let resp = server
        .get("/api/v1/keys/one-time-prekeys/count")
        .add_header(authorization_header_name(), bearer_header(at))
        .add_header(csrf_header_name(), csrf_header_value(ct))
        .await;
    if resp.status_code().is_success() {
        let body: serde_json::Value = resp.json();
        body["count"].as_u64().unwrap_or(0)
    } else {
        0
    }
}
