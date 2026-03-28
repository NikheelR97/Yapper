//! Integration tests for parental pending-request deduplication.

use super::{
    authorization_header_name, bearer_header, create_test_user_with_device, csrf_header_name,
    csrf_header_value, spawn_test_server_with_pool,
};
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
    let (_, parent_access, parent_csrf, _) =
        create_test_user_with_device(&server, &format!("parent_{suffix}")).await;
    let (requester_id, requester_access, requester_csrf, _) =
        create_test_user_with_device(&server, &format!("requester_{suffix}")).await;

    let child_username = format!("child_{suffix}");
    let child_resp = server
        .post("/api/v2/parental/children")
        .add_header(authorization_header_name(), bearer_header(&parent_access))
        .add_header(csrf_header_name(), csrf_header_value(&parent_csrf))
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
        let response = server
            .post(&request_path)
            .add_header(authorization_header_name(), bearer_header(&requester_access))
            .add_header(csrf_header_name(), csrf_header_value(&requester_csrf))
            .await;

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
