//! Integration tests for DM conversation invariants.

use super::{
    register_test_session, spawn_test_server_from_state, spawn_test_server_with_pool, TestClient,
};
use base64::Engine;
use axum_test::TestServer;
use sqlx::PgPool;
use uuid::Uuid;
use yapper_server::constants::MAX_MESSAGE_LENGTH;

async fn build_messages_test_server(pool: PgPool) -> Option<(yapper_server::AppState, TestServer)> {
    spawn_test_server_with_pool(pool).await
}

async fn create_public_server(client: &TestClient<'_>, name: &str) -> Uuid {
    let resp = client
        .post("/api/v2/servers")
        .json(&serde_json::json!({
            "name": name,
            "description": "integration messages test",
            "is_public": true,
        }))
        .await;

    assert!(resp.status_code().is_success(), "server create failed: {}", resp.text());
    let body: serde_json::Value = resp.json();
    body["id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing server id")
}

async fn general_channel_id(state: &yapper_server::AppState, server_id: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM channels WHERE server_id = $1 AND name = 'general' LIMIT 1")
        .bind(server_id)
        .fetch_one(state.db.pool())
        .await
        .expect("general channel id")
}

#[sqlx::test(migrations = "./migrations")]
async fn create_conversation_is_idempotent_for_same_pair(pool: PgPool) {
    let Some((state, server)) = build_messages_test_server(pool).await else {
        return;
    };
    let concurrent_server = spawn_test_server_from_state(state.clone());

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let sender_session = register_test_session(&server, &format!("sender_{suffix}")).await;
    let peer_session = register_test_session(&server, &format!("peer_{suffix}")).await;
    let sender_id = sender_session.user_id;
    let peer_id = peer_session.user_id;
    let sender_client = TestClient::from_session(&server, &sender_session);
    let concurrent_sender_client = TestClient::from_session(&concurrent_server, &sender_session);

    let create_path = "/api/v2/conversations";
    let create_a = async {
        sender_client
            .post(create_path)
            .json(&serde_json::json!({ "peer_id": peer_id }))
            .await
    };
    let create_b = async {
        concurrent_sender_client
            .post(create_path)
            .json(&serde_json::json!({ "peer_id": peer_id }))
            .await
    };

    let (resp_a, resp_b) = tokio::join!(create_a, create_b);
    assert!(
        resp_a.status_code().is_success(),
        "first create failed: {} {}",
        resp_a.status_code(),
        resp_a.text()
    );
    assert!(
        resp_b.status_code().is_success(),
        "second create failed: {} {}",
        resp_b.status_code(),
        resp_b.text()
    );

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

#[sqlx::test(migrations = "./migrations")]
async fn channel_message_rejects_ciphertext_over_max_length(pool: PgPool) {
    let Some((state, server)) = build_messages_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let sender_session = register_test_session(&server, &format!("channel_sender_{suffix}")).await;
    let sender_client = TestClient::from_session(&server, &sender_session);
    let server_id = create_public_server(&sender_client, &format!("Channel {suffix}")).await;
    let channel_id = general_channel_id(&state, server_id).await;

    let oversized = base64::engine::general_purpose::STANDARD.encode(vec![0_u8; MAX_MESSAGE_LENGTH + 1]);
    let resp = sender_client
        .post(&format!("/api/v2/channels/{channel_id}/messages"))
        .json(&serde_json::json!({
            "ciphertext": oversized,
            "message_type": "text"
        }))
        .await;

    assert_eq!(resp.status_code().as_u16(), 400);
    assert!(resp.text().contains("Ciphertext exceeds size limit"), "unexpected response: {}", resp.text());
}

#[sqlx::test(migrations = "./migrations")]
async fn dm_message_rejects_ciphertext_over_max_length(pool: PgPool) {
    let Some(server) = build_messages_test_server(pool).await.map(|(_, server)| server) else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let sender_session = register_test_session(&server, &format!("dm_sender_{suffix}")).await;
    let peer_session = register_test_session(&server, &format!("dm_peer_{suffix}")).await;
    let sender_client = TestClient::from_session(&server, &sender_session);
    let sender_id = sender_session.user_id;
    let peer_id = peer_session.user_id;
    let peer_device_id = peer_session.device_id.expect("missing peer device id");

    let create = sender_client
        .post("/api/v2/conversations")
        .json(&serde_json::json!({ "peer_id": peer_id }))
        .await;
    assert!(create.status_code().is_success(), "conversation create failed: {}", create.text());
    let body: serde_json::Value = create.json();
    let conv_id: Uuid = body["id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing conversation id");

    let oversized = base64::engine::general_purpose::STANDARD.encode(vec![1_u8; MAX_MESSAGE_LENGTH + 1]);
    let send = sender_client
        .post(&format!("/api/v2/conversations/{conv_id}/messages"))
        .json(&serde_json::json!({
            "envelopes": [{
                "recipient_user_id": peer_id,
                "recipient_device_id": peer_device_id,
                "ciphertext": oversized,
                "msg_num": 1
            }]
        }))
        .await;

    assert_eq!(send.status_code().as_u16(), 400);
    assert!(send.text().contains("Ciphertext exceeds size limit"), "unexpected response: {}", send.text());

    let _ = sender_id;
}

#[sqlx::test(migrations = "./migrations")]
async fn channel_message_rows_without_content_are_rejected_by_the_db_constraint(pool: PgPool) {
    let Some((state, server)) = build_messages_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let sender_session = register_test_session(&server, &format!("channel_constraint_{suffix}")).await;
    let sender_client = TestClient::from_session(&server, &sender_session);
    let sender_id = sender_session.user_id;
    let server_id = create_public_server(&sender_client, &format!("Constraint {suffix}")).await;
    let channel_id = general_channel_id(&state, server_id).await;

    let result = sqlx::query(
        "INSERT INTO messages (id, channel_id, sender_id, message_type, delivered) \
         VALUES ($1, $2, $3, 'text', FALSE)",
    )
    .bind(Uuid::new_v4())
    .bind(channel_id)
    .bind(sender_id)
    .execute(state.db.pool())
    .await;

    let err = result.expect_err("channel row without content should violate the message content constraint");
    let db_err = err
        .as_database_error()
        .expect("constraint violation should surface as a database error");
    assert_eq!(db_err.constraint(), Some("messages_ciphertext_xor_plaintext"));
}
