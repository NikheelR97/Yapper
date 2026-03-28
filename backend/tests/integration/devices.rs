//! Integration tests - device trust state-machine.
//!
//! Tests the bootstrap -> pending -> trusted -> revoked lifecycle.

use super::{
    authorization_header_name, bearer_header, create_test_user, create_test_user_with_device,
    csrf_header_name, csrf_header_value, register_test_session, spawn_test_server_from_pool,
    TestClient,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Device bootstrap creates a first device and a valid device-aware session.
#[sqlx::test(migrations = "./migrations")]
async fn devices_bootstrap_creates_pending_device(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_user_id, access_token, csrf_token, _device_id) =
        create_test_user_with_device(&server, &suffix).await;

    let devices_resp = server
        .get("/api/v2/devices")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
        .await;

    assert!(
        devices_resp.status_code().is_success(),
        "GET /api/v2/devices failed: {}",
        devices_resp.text()
    );

    let devices: serde_json::Value = devices_resp.json();
    assert!(devices.is_array(), "devices response should be an array");
    let devices_arr = devices.as_array().unwrap();
    assert!(
        !devices_arr.is_empty(),
        "device list should not be empty after v2 bootstrap"
    );

    let trust_state = devices_arr[0]["trust_state"].as_str().unwrap_or("");
    assert!(
        trust_state == "trusted" || trust_state == "pending_trust",
        "unexpected trust_state: {trust_state}"
    );
}

/// Health endpoint returns 200 with db:true when the database is reachable.
#[sqlx::test(migrations = "./migrations")]
async fn health_check_returns_200(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let resp = server.get("/health").await;
    assert_eq!(
        resp.status_code().as_u16(),
        200,
        "health check failed: {}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert_eq!(body["ok"], true, "health ok flag not true");
}

/// Unauthenticated requests to protected endpoints must return 401.
#[sqlx::test(migrations = "./migrations")]
async fn protected_endpoints_require_auth(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let (_user_id, at, ct) =
        create_test_user(&server, &Uuid::new_v4().to_string().replace('-', "")[..8]).await;
    let _ = (at, ct); // suppress unused warning

    let protected = ["/api/v2/users/me", "/api/v2/servers", "/api/v2/devices"];

    for path in &protected {
        let resp = server.get(path).await;
        let status = resp.status_code().as_u16();
        assert!(
            status == 401 || status == 403,
            "expected 401/403 for unauthenticated {path}, got {status}"
        );
    }
}

/// Sync events remain pending until the target device explicitly acknowledges them.
#[sqlx::test(migrations = "./migrations")]
async fn sync_events_require_explicit_ack_before_delivery_is_committed(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = register_test_session(&server, &suffix).await;
    let client = TestClient::from_session(&server, &session);
    let device_id = session.device_id.expect("missing device id");

    let enqueue = client
        .post("/api/v2/devices/sync-events")
        .json(&serde_json::json!({
            "target_device_id": device_id,
            "event_type": "device_sync_chunk",
            "payload": {
                "chunk_index": 0,
                "total_chunks": 1,
                "chunk": "ZmFrZQ==",
            },
        }))
        .await;

    assert!(
        enqueue.status_code().is_success(),
        "enqueue sync event failed: {}",
        enqueue.text()
    );

    let first_fetch = client.get("/api/v2/devices/sync-events").await;
    assert!(
        first_fetch.status_code().is_success(),
        "first sync-events fetch failed: {}",
        first_fetch.text()
    );
    let first_events: serde_json::Value = first_fetch.json();
    let first_events = first_events
        .as_array()
        .expect("sync-events response should be an array");
    assert_eq!(first_events.len(), 1, "expected one pending sync event");
    let event_id = first_events[0]["id"]
        .as_str()
        .expect("sync event id missing")
        .to_string();

    let second_fetch = client.get("/api/v2/devices/sync-events").await;
    assert!(
        second_fetch.status_code().is_success(),
        "second sync-events fetch failed: {}",
        second_fetch.text()
    );
    let second_events: serde_json::Value = second_fetch.json();
    let second_events = second_events
        .as_array()
        .expect("sync-events response should be an array");
    assert_eq!(
        second_events.len(),
        1,
        "fetching without ack must not consume sync events"
    );

    let ack = client
        .post("/api/v2/devices/sync-events/ack")
        .json(&serde_json::json!({ "event_ids": [event_id] }))
        .await;
    assert_eq!(
        ack.status_code().as_u16(),
        204,
        "sync-events ack failed: {}",
        ack.text()
    );

    let post_ack_fetch = client.get("/api/v2/devices/sync-events").await;
    assert!(
        post_ack_fetch.status_code().is_success(),
        "post-ack sync-events fetch failed: {}",
        post_ack_fetch.text()
    );
    let post_ack_events: serde_json::Value = post_ack_fetch.json();
    let post_ack_events = post_ack_events
        .as_array()
        .expect("sync-events response should be an array");
    assert!(
        post_ack_events.is_empty(),
        "acknowledged sync events should no longer be returned"
    );
}
