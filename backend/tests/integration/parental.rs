//! Integration tests for parental pending-request deduplication.

use super::{register_test_session, spawn_test_server_with_pool, TestClient};
use axum_test::TestServer;
use sqlx::PgPool;
use uuid::Uuid;

async fn build_parental_test_server(pool: PgPool) -> Option<(yapper_server::AppState, TestServer)> {
    spawn_test_server_with_pool(pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_pending_friend_requests_are_deduplicated(pool: PgPool) {
    let Some((state, server)) = build_parental_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let parent_session = register_test_session(&server, &format!("parent_{suffix}")).await;
    let requester_session = register_test_session(&server, &format!("requester_{suffix}")).await;
    let parent_client = TestClient::from_session(&server, &parent_session);
    let requester_client = TestClient::from_session(&server, &requester_session);
    let requester_id = requester_session.user_id;

    let child_username = format!("child_{suffix}");
    let child_resp = parent_client
        .post("/api/v2/parental/children")
        .json(&serde_json::json!({
            "username": &child_username,
            "display_name": "Pending Child",
            "email": format!("child_{suffix}@integration.test"),
            "password": format!("ChildPass123!{suffix}"),
            "date_of_birth": "2015-03-26",
        }))
        .await;
    assert!(child_resp.status_code().is_success(), "child creation failed: {}", child_resp.text());

    let request_path = format!("/api/v2/users/by/{child_username}/friend-request");
    for _ in 0..2 {
        let response = requester_client.post(&request_path).await;

        assert!(
            response.status_code().is_success() || response.status_code().as_u16() == 202,
            "friend request failed: {} - {}",
            response.status_code(),
            response.text()
        );
    }

    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pending_friend_requests \
         WHERE child_user_id = (SELECT id FROM users WHERE username = $1) \
           AND requester_id = $2 \
           AND status = 'pending'",
    )
    .bind(&child_username)
    .bind(requester_id)
    .fetch_one(state.db.pool())
    .await
    .expect("pending friend count");

    assert_eq!(pending_count, 1, "duplicate pending friend requests should collapse to one row");
}
