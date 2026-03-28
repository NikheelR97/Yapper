//! Integration tests for presence privacy and read-receipt authorization.

use super::{
    create_test_user, register_test_session, spawn_test_server_with_pool, TestClient,
};
use axum_test::TestServer;
use sqlx::PgPool;
use uuid::Uuid;
use yapper_server::hub::load_read_receipt_member_ids;

async fn build_privacy_test_server(pool: PgPool) -> Option<(yapper_server::AppState, TestServer)> {
    spawn_test_server_with_pool(pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn presence_hides_last_seen_for_peers_and_keeps_self_view(pool: PgPool) {
    let Some((state, server)) = build_privacy_test_server(pool).await else {
        return;
    };

    let suffix_a = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let suffix_b = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let viewer_session = register_test_session(&server, &suffix_a).await;
    let target_session = register_test_session(&server, &suffix_b).await;
    let viewer_client = TestClient::from_session(&server, &viewer_session);
    let target_client = TestClient::from_session(&server, &target_session);
    let target_id = target_session.user_id;

    sqlx::query(
        "UPDATE users SET last_seen_at = TIMESTAMPTZ '2026-03-21 12:00:00+00' WHERE id = $1",
    )
    .bind(target_id)
    .execute(state.db.pool())
    .await
    .expect("set last_seen_at");

    // Regression note: this used to 500 when the first privacy write inserted
    // NULL into NOT NULL columns in `user_privacy_settings`.
    let privacy_update = target_client
        .patch("/api/v2/users/me/privacy")
        .json(&serde_json::json!({
            "show_last_seen": false
        }))
        .await;
    assert_eq!(
        privacy_update.status_code().as_u16(),
        204,
        "privacy update failed: {}",
        privacy_update.text()
    );

    let peer_path = format!("/api/v2/users/{target_id}/presence");
    let peer_resp = viewer_client.get(&peer_path).await;
    assert!(
        peer_resp.status_code().is_success(),
        "peer presence failed: {}",
        peer_resp.text()
    );
    let peer_body: serde_json::Value = peer_resp.json();
    assert!(
        peer_body["last_seen_at"].is_null(),
        "peer should not see last_seen_at when disabled"
    );

    let self_path = format!("/api/v2/users/{target_id}/presence");
    let self_resp = target_client.get(&self_path).await;
    assert!(
        self_resp.status_code().is_success(),
        "self presence failed: {}",
        self_resp.text()
    );
    let self_body: serde_json::Value = self_resp.json();
    assert!(
        self_body["last_seen_at"].is_string(),
        "self view should retain last_seen_at even when disabled"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn read_receipt_helper_requires_the_message_real_channel(pool: PgPool) {
    let Some((state, server)) = build_privacy_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, _access_token, _csrf_token) = create_test_user(&server, &suffix).await;
    let server_id = Uuid::new_v4();
    let real_channel_id = Uuid::new_v4();
    let wrong_channel_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();

    sqlx::query("INSERT INTO servers (id, name, slug, owner_id) VALUES ($1, $2, $3, $4)")
        .bind(server_id)
        .bind(format!("Privacy Test {suffix}"))
        .bind(format!("privacy-test-{suffix}"))
        .bind(user_id)
        .execute(state.db.pool())
        .await
        .expect("insert server");

    sqlx::query(
        "INSERT INTO server_memberships (user_id, server_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(user_id)
    .bind(server_id)
    .execute(state.db.pool())
    .await
    .expect("insert membership");

    sqlx::query(
        "INSERT INTO channels (id, server_id, name, type, position) VALUES ($1, $2, $3, 'text', 0)",
    )
    .bind(real_channel_id)
    .bind(server_id)
    .bind(format!("real-{suffix}"))
    .execute(state.db.pool())
    .await
    .expect("insert real channel");

    sqlx::query(
        "INSERT INTO channels (id, server_id, name, type, position) VALUES ($1, $2, $3, 'text', 1)",
    )
    .bind(wrong_channel_id)
    .bind(server_id)
    .bind(format!("wrong-{suffix}"))
    .execute(state.db.pool())
    .await
    .expect("insert wrong channel");

    sqlx::query(
        "INSERT INTO messages (id, channel_id, sender_id, ciphertext, message_type, delivered)
         VALUES ($1, $2, $3, $4, 'text', FALSE)",
    )
    .bind(message_id)
    .bind(real_channel_id)
    .bind(user_id)
    .bind(vec![1_u8, 2, 3, 4])
    .execute(state.db.pool())
    .await
    .expect("insert channel message");

    let correct_members =
        load_read_receipt_member_ids(message_id, real_channel_id, user_id, &state).await;
    let correct_members =
        correct_members.expect("expected receipt helper to accept the real channel");
    assert!(correct_members.contains(&user_id));

    let wrong_members =
        load_read_receipt_member_ids(message_id, wrong_channel_id, user_id, &state).await;
    assert!(
        wrong_members.is_none(),
        "expected receipt helper to reject a mismatched channel"
    );
}
