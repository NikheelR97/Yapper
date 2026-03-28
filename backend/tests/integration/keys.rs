//! Integration tests for key server endpoints.
//!
//! Verifies v2 identity/signed-prekey/OPK upload, bundle fetch, OPK
//! consumption, and that removed legacy write routes are no longer mounted.

use super::{spawn_test_server_from_pool, TestClient};
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;
use yapper_server::constants::MAX_OPK_BATCH;

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
    _server: &axum_test::TestServer,
    client: &TestClient<'_>,
    dh_public_key: &str,
    signing_public_key: &str,
) {
    let response = client
        .post("/api/v2/keys/identity")
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
    _server: &axum_test::TestServer,
    client: &TestClient<'_>,
    key_id: i32,
    public_key: &str,
    signature: &str,
) {
    let response = client
        .post("/api/v2/keys/signed-prekey")
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
    _server: &axum_test::TestServer,
    client: &TestClient<'_>,
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

    let response = client
        .post("/api/v2/keys/one-time-prekeys")
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

async fn opk_count(client: &TestClient<'_>) -> u64 {
    let response = client.get("/api/v2/keys/one-time-prekey-count").await;

    assert!(
        response.status_code().is_success(),
        "OPK count failed: {} - {}",
        response.status_code(),
        response.text()
    );

    let body: serde_json::Value = response.json();
    body["count"].as_u64().unwrap_or(0)
}

async fn prepare_bundle_material(
    server: &axum_test::TestServer,
    client: &TestClient<'_>,
    dh_seed: u8,
    signing_seed: u8,
    signed_prekey_id: i32,
    signed_prekey_seed: u8,
    opk_start: i32,
    opk_count: i32,
) {
    let identity_dh_key = x25519_key_b64(dh_seed);
    let identity_signing_key = signing_key(signing_seed);
    let identity_signing_public =
        base64::engine::general_purpose::STANDARD.encode(identity_signing_key.verifying_key().to_bytes());

    upload_identity(
        server,
        client,
        &identity_dh_key,
        &identity_signing_public,
    )
    .await;

    let signed_prekey_raw = [signed_prekey_seed; 32];
    let signed_prekey = base64::engine::general_purpose::STANDARD.encode(signed_prekey_raw);
    let signed_prekey_signature = signature_b64(&identity_signing_key, &signed_prekey_raw);
    upload_signed_prekey(
        server,
        client,
        signed_prekey_id,
        &signed_prekey,
        &signed_prekey_signature,
    )
    .await;

    if opk_count > 0 {
        upload_opks(server, client, opk_start, opk_count).await;
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn legacy_v1_key_write_routes_are_unmounted(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    for (path, body) in [
        (
            "/api/v1/keys/identity",
            serde_json::json!({
                "device_id": 1,
                "dh_public_key": x25519_key_b64(1),
                "signing_public_key": x25519_key_b64(2),
            }),
        ),
        (
            "/api/v1/keys/signed-prekey",
            serde_json::json!({
                "device_id": 1,
                "key_id": 1,
                "public_key": x25519_key_b64(3),
                "signature": base64::engine::general_purpose::STANDARD.encode([7_u8; 64]),
            }),
        ),
        (
            "/api/v1/keys/one-time-prekeys",
            serde_json::json!({
                "device_id": 1,
                "keys": [{ "key_id": 1, "public_key": x25519_key_b64(4) }],
            }),
        ),
    ] {
        let response = server.post(path).json(&body).await;

        assert_eq!(
            response.status_code().as_u16(),
            404,
            "legacy key write route should be unmounted: {path}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn keys_upload_rejects_batches_over_the_maximum_size(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = super::register_test_session(&server, &format!("opk_batch_{suffix}")).await;
    let client = TestClient::from_session(&server, &session);

    let keys: Vec<serde_json::Value> = (0..(MAX_OPK_BATCH + 1))
        .map(|offset| {
            serde_json::json!({
                "key_id": offset as i32,
                "public_key": x25519_key_b64((offset % 250) as u8),
            })
        })
        .collect();

    let response = client
        .post("/api/v2/keys/one-time-prekeys")
        .json(&serde_json::json!({
            "device_id": 9999,
            "keys": keys,
        }))
        .await;

    assert_eq!(response.status_code().as_u16(), 400);
    let body: serde_json::Value = response.json();
    // Current handler text is "Provide 1–100 one-time prekeys" (en dash may vary by encoding).
    let error_text = body["message"]
        .as_str()
        .or_else(|| body["error"].as_str())
        .unwrap_or("");
    assert!(
        error_text.contains("one-time prekeys")
            && (error_text.contains("1") || error_text.contains(&MAX_OPK_BATCH.to_string())),
        "unexpected rejection: {body}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn keys_bundle_response_exposes_only_public_key_material(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = super::register_test_session(&server, &format!("bundle_public_{suffix}")).await;
    let user_id = session.user_id;
    let client = TestClient::from_session(&server, &session);

    prepare_bundle_material(&server, &client, 1, 9, 1, 5, 100, 2).await;

    let bundle = client.get(&format!("/api/v2/keys/{user_id}/bundles")).await;

    assert!(bundle.status_code().is_success(), "bundle fetch failed: {}", bundle.text());

    let body: serde_json::Value = bundle.json();
    let bundles = body.as_array().expect("bundle response should be an array");
    assert_eq!(bundles.len(), 1);
    let bundle_obj = bundles[0].as_object().expect("bundle should be an object");

    let expected: HashSet<&str> = [
        "user_id",
        "device_id",
        "signal_device_id",
        "identity_dh_key",
        "identity_sig_key",
        "signed_prekey_id",
        "signed_prekey",
        "signed_prekey_sig",
        "one_time_prekey_id",
        "one_time_prekey",
    ]
    .into_iter()
    .collect();
    let actual: HashSet<&str> = bundle_obj.keys().map(|key| key.as_str()).collect();

    assert_eq!(actual, expected, "bundle leaked unexpected fields: {bundle_obj:?}");
    assert!(
        bundle_obj.keys().all(|key| !key.contains("private")),
        "bundle response leaked private key material: {bundle_obj:?}"
    );
    assert_eq!(bundle_obj["user_id"], user_id.to_string());
    assert!(bundle_obj["identity_dh_key"].is_string());
    assert!(bundle_obj["identity_sig_key"].is_string());
    assert!(bundle_obj["signed_prekey"].is_string());
    assert!(bundle_obj["signed_prekey_sig"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn keys_opk_consumption_is_atomic_under_concurrent_bundle_requests(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool.clone()).await else {
        return;
    };
    let Some(concurrent_server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = super::register_test_session(&server, &format!("bundle_atomic_{suffix}")).await;
    let user_id = session.user_id;
    let client = TestClient::from_session(&server, &session);
    let concurrent_client = TestClient::from_session(&concurrent_server, &session);

    prepare_bundle_material(&server, &client, 11, 12, 7, 13, 200, 1).await;

    let bundle_path = format!("/api/v2/keys/{user_id}/bundles");
    let first = async {
        client.get(&bundle_path).add_query_param("consume_opk", "true").await
    };
    let second = async {
        concurrent_client
            .get(&bundle_path)
            .add_query_param("consume_opk", "true")
            .await
    };

    let (resp_a, resp_b) = tokio::join!(first, second);
    assert!(
        resp_a.status_code().is_success(),
        "first bundle fetch failed: {} - {}",
        resp_a.status_code(),
        resp_a.text()
    );
    assert!(
        resp_b.status_code().is_success(),
        "second bundle fetch failed: {} - {}",
        resp_b.status_code(),
        resp_b.text()
    );

    let bodies = [resp_a.json::<serde_json::Value>(), resp_b.json::<serde_json::Value>()];
    let mut consumed = 0;
    for body in bodies {
        let bundles = body.as_array().expect("bundle response should be an array");
        assert_eq!(bundles.len(), 1, "expected one bundle per request");
        if bundles[0]["one_time_prekey"].is_string() {
            consumed += 1;
        }
    }
    assert_eq!(consumed, 1, "exactly one concurrent request should consume the only OPK");

    let remaining = opk_count(&client).await;
    assert_eq!(remaining, 0, "the only OPK should be marked consumed");
}

#[sqlx::test(migrations = "./migrations")]
async fn keys_upload_and_fetch_bundle_v2(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = super::register_test_session(&server, &suffix).await;
    let user_id = session.user_id;
    let client = TestClient::from_session(&server, &session);

    let identity_dh_key = x25519_key_b64(1);
    let identity_signing_key = signing_key(9);
    let identity_signing_public =
        base64::engine::general_purpose::STANDARD.encode(identity_signing_key.verifying_key().to_bytes());

    upload_identity(
        &server,
        &client,
        &identity_dh_key,
        &identity_signing_public,
    )
    .await;

    let signed_prekey_raw = [5_u8; 32];
    let signed_prekey = base64::engine::general_purpose::STANDARD.encode(signed_prekey_raw);
    let signed_prekey_signature = signature_b64(&identity_signing_key, &signed_prekey_raw);
    upload_signed_prekey(
        &server,
        &client,
        1,
        &signed_prekey,
        &signed_prekey_signature,
    )
    .await;

    upload_opks(&server, &client, 100, 5).await;

    let bundle = client
        .get(&format!("/api/v2/keys/{user_id}/bundles"))
        .add_query_param("consume_opk", "true")
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

#[sqlx::test(migrations = "./migrations")]
async fn keys_opk_count_decrements_after_consumption_v2(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = super::register_test_session(&server, &suffix).await;
    let user_id = session.user_id;
    let client = TestClient::from_session(&server, &session);

    let identity_dh_key = x25519_key_b64(11);
    let identity_signing_key = signing_key(12);
    let identity_signing_public =
        base64::engine::general_purpose::STANDARD.encode(identity_signing_key.verifying_key().to_bytes());

    upload_identity(
        &server,
        &client,
        &identity_dh_key,
        &identity_signing_public,
    )
    .await;

    let signed_prekey_raw = [13_u8; 32];
    let signed_prekey = base64::engine::general_purpose::STANDARD.encode(signed_prekey_raw);
    let signed_prekey_signature = signature_b64(&identity_signing_key, &signed_prekey_raw);
    upload_signed_prekey(
        &server,
        &client,
        7,
        &signed_prekey,
        &signed_prekey_signature,
    )
    .await;

    upload_opks(&server, &client, 200, 3).await;

    let count_before = opk_count(&client).await;

    let bundle = client
        .get(&format!("/api/v2/keys/{user_id}/bundles"))
        .add_query_param("consume_opk", "true")
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
        let count_after = opk_count(&client).await;
        assert!(
            count_after < count_before,
            "OPK count did not decrease after consumption: before={count_before}, after={count_after}"
        );
    }
}
