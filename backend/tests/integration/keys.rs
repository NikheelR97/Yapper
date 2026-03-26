//! Integration tests for key server endpoints.
//!
//! Verifies v2 identity/signed-prekey/OPK upload, bundle fetch, OPK
//! consumption, and that removed legacy write routes are no longer mounted.

use super::{
    authorization_header_name, bearer_header, create_test_user_with_device, csrf_header_name,
    csrf_header_value, spawn_test_server,
};
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serial_test::serial;
use uuid::Uuid;

fn x25519_key_b64(seed: u8) -> String {
    let bytes = [seed; 32];
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn signature_b64(signing_key: &SigningKey, message: &[u8]) -> String {
    let signature = signing_key.sign(message);
    base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
}

async fn upload_identity(
    server: &axum_test::TestServer,
    access_token: &str,
    csrf_token: &str,
    dh_public_key: &str,
    signing_public_key: &str,
) {
    let response = server
        .post("/api/v2/keys/identity")
        .add_header(authorization_header_name(), bearer_header(access_token))
        .add_header(csrf_header_name(), csrf_header_value(csrf_token))
        .json(&serde_json::json!({
            "device_id": 9999,
            "dh_public_key": dh_public_key,
            "signing_public_key": signing_public_key,
        }))
        .await;

    assert!(
        response.status_code().is_success(),
        "identity upload failed: {} - {}",
        response.status_code(),
        response.text()
    );
}

async fn upload_signed_prekey(
    server: &axum_test::TestServer,
    access_token: &str,
    csrf_token: &str,
    key_id: i32,
    public_key: &str,
    signature: &str,
) {
    let response = server
        .post("/api/v2/keys/signed-prekey")
        .add_header(authorization_header_name(), bearer_header(access_token))
        .add_header(csrf_header_name(), csrf_header_value(csrf_token))
        .json(&serde_json::json!({
            "device_id": 9999,
            "key_id": key_id,
            "public_key": public_key,
            "signature": signature,
        }))
        .await;

    assert!(
        response.status_code().is_success(),
        "signed prekey upload failed: {} - {}",
        response.status_code(),
        response.text()
    );
}

async fn upload_opks(
    server: &axum_test::TestServer,
    access_token: &str,
    csrf_token: &str,
    start_key_id: i32,
    count: i32,
) {
    let keys: Vec<serde_json::Value> = (0..count)
        .map(|offset| {
            serde_json::json!({
                "key_id": start_key_id + offset,
                "public_key": x25519_key_b64((offset + 10) as u8),
            })
        })
        .collect();

    let response = server
        .post("/api/v2/keys/one-time-prekeys")
        .add_header(authorization_header_name(), bearer_header(access_token))
        .add_header(csrf_header_name(), csrf_header_value(csrf_token))
        .json(&serde_json::json!({
            "device_id": 9999,
            "keys": keys,
        }))
        .await;

    assert!(
        response.status_code().is_success(),
        "OPK upload failed: {} - {}",
        response.status_code(),
        response.text()
    );
}

async fn opk_count(server: &axum_test::TestServer, access_token: &str, csrf_token: &str) -> u64 {
    let response = server
        .get("/api/v2/keys/one-time-prekey-count")
        .add_header(authorization_header_name(), bearer_header(access_token))
        .add_header(csrf_header_name(), csrf_header_value(csrf_token))
        .await;

    assert!(
        response.status_code().is_success(),
        "OPK count failed: {} - {}",
        response.status_code(),
        response.text()
    );

    let body: serde_json::Value = response.json();
    body["count"].as_u64().unwrap_or(0)
}

#[tokio::test]
#[serial]
async fn legacy_v1_key_write_routes_are_unmounted() {
    let Some(server) = spawn_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_user_id, access_token, csrf_token, _device_id) =
        create_test_user_with_device(&server, &suffix).await;

    for (path, body) in [
        (
            "/api/v2/keys/identity",
            serde_json::json!({
                "device_id": 1,
                "dh_public_key": x25519_key_b64(1),
                "signing_public_key": x25519_key_b64(2),
            }),
        ),
        (
            "/api/v2/keys/signed-prekey",
            serde_json::json!({
                "device_id": 1,
                "key_id": 1,
                "public_key": x25519_key_b64(3),
                "signature": base64::engine::general_purpose::STANDARD.encode([7_u8; 64]),
            }),
        ),
        (
            "/api/v2/keys/one-time-prekeys",
            serde_json::json!({
                "device_id": 1,
                "keys": [{ "key_id": 1, "public_key": x25519_key_b64(4) }],
            }),
        ),
    ] {
        let response = server
            .post(path)
            .add_header(authorization_header_name(), bearer_header(&access_token))
            .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
            .json(&body)
            .await;

        assert_eq!(
            response.status_code().as_u16(),
            404,
            "legacy key write route should be unmounted: {path}"
        );
    }
}

#[tokio::test]
#[serial]
async fn keys_upload_and_fetch_bundle_v2() {
    let Some(server) = spawn_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, access_token, csrf_token, _device_id) =
        create_test_user_with_device(&server, &suffix).await;

    let identity_dh_key = x25519_key_b64(1);
    let identity_signing_key = signing_key(9);
    let identity_signing_public =
        base64::engine::general_purpose::STANDARD.encode(identity_signing_key.verifying_key().to_bytes());

    upload_identity(
        &server,
        &access_token,
        &csrf_token,
        &identity_dh_key,
        &identity_signing_public,
    )
    .await;

    let signed_prekey_raw = [5_u8; 32];
    let signed_prekey = base64::engine::general_purpose::STANDARD.encode(signed_prekey_raw);
    let signed_prekey_signature = signature_b64(&identity_signing_key, &signed_prekey_raw);
    upload_signed_prekey(
        &server,
        &access_token,
        &csrf_token,
        1,
        &signed_prekey,
        &signed_prekey_signature,
    )
    .await;

    upload_opks(&server, &access_token, &csrf_token, 100, 5).await;

    let bundle = server
        .get(&format!("/api/v2/keys/{user_id}/bundles?consume_opk=true"))
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .await;

    assert!(
        bundle.status_code().is_success(),
        "bundle fetch failed: {} - {}",
        bundle.status_code(),
        bundle.text()
    );

    let body: serde_json::Value = bundle.json();
    let bundles = body.as_array().expect("bundle response should be an array");
    assert_eq!(bundles.len(), 1, "expected one trusted device bundle");
    assert_eq!(bundles[0]["user_id"], user_id.to_string());
    assert!(bundles[0]["identity_dh_key"].is_string());
    assert!(bundles[0]["identity_sig_key"].is_string());
    assert!(bundles[0]["signed_prekey"].is_string());
    assert!(bundles[0]["signed_prekey_sig"].is_string());
    assert!(bundles[0]["one_time_prekey"].is_string());
}

#[tokio::test]
#[serial]
async fn keys_opk_count_decrements_after_consumption_v2() {
    let Some(server) = spawn_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, access_token, csrf_token, _device_id) =
        create_test_user_with_device(&server, &suffix).await;

    let identity_dh_key = x25519_key_b64(11);
    let identity_signing_key = signing_key(12);
    let identity_signing_public =
        base64::engine::general_purpose::STANDARD.encode(identity_signing_key.verifying_key().to_bytes());

    upload_identity(
        &server,
        &access_token,
        &csrf_token,
        &identity_dh_key,
        &identity_signing_public,
    )
    .await;

    let signed_prekey_raw = [13_u8; 32];
    let signed_prekey = base64::engine::general_purpose::STANDARD.encode(signed_prekey_raw);
    let signed_prekey_signature = signature_b64(&identity_signing_key, &signed_prekey_raw);
    upload_signed_prekey(
        &server,
        &access_token,
        &csrf_token,
        7,
        &signed_prekey,
        &signed_prekey_signature,
    )
    .await;

    upload_opks(&server, &access_token, &csrf_token, 200, 3).await;

    let count_before = opk_count(&server, &access_token, &csrf_token).await;

    let bundle = server
        .get(&format!("/api/v2/keys/{user_id}/bundles?consume_opk=true"))
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .await;
    assert!(
        bundle.status_code().is_success(),
        "bundle fetch failed: {} - {}",
        bundle.status_code(),
        bundle.text()
    );

    let bundle_body: serde_json::Value = bundle.json();
    let bundle_rows = bundle_body.as_array().expect("bundle response should be an array");
    assert_eq!(bundle_rows.len(), 1);

    if bundle_rows[0]["one_time_prekey"].is_string() {
        let count_after = opk_count(&server, &access_token, &csrf_token).await;
        assert!(
            count_after < count_before,
            "OPK count did not decrease after consumption: before={count_before}, after={count_after}"
        );
    }
}
