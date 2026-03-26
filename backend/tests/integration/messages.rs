//! Integration tests for DM conversation invariants.

use super::{
    authorization_header_name, bearer_header, build_test_state, create_test_user_with_device,
    csrf_header_name, csrf_header_value,
};
use axum_test::TestServer;
use serial_test::serial;
use uuid::Uuid;
use yapper_server::build_router;

async fn build_messages_test_server() -> Option<(yapper_server::AppState, TestServer)> {
    let state = build_test_state().await?;
    let server = TestServer::new(build_router(state.clone())).expect("Failed to create test server");
    Some((state, server))
}

#[tokio::test]
#[serial]
async fn create_conversation_is_idempotent_for_same_pair() {
    let Some((state, server)) = build_messages_test_server().await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (sender_id, access_token, csrf_token, _) =
        create_test_user_with_device(&server, &format!("sender_{suffix}")).await;
    let (peer_id, _, _, _) =
        create_test_user_with_device(&server, &format!("peer_{suffix}")).await;

    let create_path = "/api/v2/conversations/";
    let create_a = async {
        server
            .post(create_path)
            .add_header(authorization_header_name(), bearer_header(&access_token))
            .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
            .json(&serde_json::json!({ "peer_id": peer_id }))
            .await
    };
    let create_b = async {
        server
            .post(create_path)
            .add_header(authorization_header_name(), bearer_header(&access_token))
            .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
            .json(&serde_json::json!({ "peer_id": peer_id }))
            .await
    };

    let (resp_a, resp_b) = tokio::join!(create_a, create_b);
    assert!(resp_a.status_code().is_success(), "first create failed: {}", resp_a.text());
    assert!(resp_b.status_code().is_success(), "second create failed: {}", resp_b.text());

    let body_a: serde_json::Value = resp_a.json();
    let body_b: serde_json::Value = resp_b.json();
    assert_eq!(body_a["id"], body_b["id"], "same pair should resolve to one conversation");

    let (user_low, user_high) = if sender_id < peer_id {
        (sender_id, peer_id)
    } else {
        (peer_id, sender_id)
    };

    let pair_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dm_conversation_pairs WHERE user_low = $1 AND user_high = $2",
    )
    .bind(user_low)
    .bind(user_high)
    .fetch_one(state.db.pool())
    .await
    .expect("pair count");

    assert_eq!(pair_count, 1, "same pair should have exactly one canonical pair row");
}
