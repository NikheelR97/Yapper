//! Integration tests for screentime authorization.

use super::{register_test_session, spawn_test_server_with_pool, TestClient};
use axum_test::TestServer;
use sqlx::PgPool;
use uuid::Uuid;

async fn build_screentime_test_server(pool: PgPool) -> Option<(yapper_server::AppState, TestServer)> {
    spawn_test_server_with_pool(pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn parental_screentime_access_requires_the_linked_parent(pool: PgPool) {
    let Some((_state, server)) = build_screentime_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let parent_a_session = register_test_session(&server, &format!("screen_parent_a_{suffix}")).await;
    let parent_b_session = register_test_session(&server, &format!("screen_parent_b_{suffix}")).await;
    let parent_a_client = TestClient::from_session(&server, &parent_a_session);
    let parent_b_client = TestClient::from_session(&server, &parent_b_session);

    let child_resp = parent_a_client
        .post("/api/v2/parental/children")
        .json(&serde_json::json!({
            "username": format!("screen_child_{suffix}"),
            "display_name": "Screen Child",
            "email": format!("screen_child_{suffix}@integration.test"),
            "password": format!("ChildPass123!{suffix}"),
            "date_of_birth": "2015-03-26",
        }))
        .await;

    assert!(child_resp.status_code().is_success(), "child creation failed: {}", child_resp.text());
    let child_body: serde_json::Value = child_resp.json();
    let child_id: Uuid = child_body["child_id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing child id");

    let forbidden = parent_b_client
        .get(&format!("/api/v2/parental/children/{child_id}/screentime"))
        .add_query_param("period", "today")
        .await;
    assert_eq!(forbidden.status_code().as_u16(), 403, "wrong parent should be rejected");

    let allowed = parent_a_client
        .get(&format!("/api/v2/parental/children/{child_id}/screentime"))
        .add_query_param("period", "today")
        .await;
    assert!(allowed.status_code().is_success(), "linked parent should be allowed: {}", allowed.text());
}

#[sqlx::test(migrations = "./migrations")]
async fn screentime_reports_reject_future_recorded_dates(pool: PgPool) {
    let Some((_state, server)) = build_screentime_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = register_test_session(&server, &format!("screen_future_{suffix}")).await;
    let client = TestClient::from_session(&server, &session);
    let future_date = (chrono::Utc::now().date_naive() + chrono::Duration::days(1)).to_string();

    let response = client
        .post("/api/v2/screentime/report")
        .json(&serde_json::json!({
            "recordedDate": future_date,
            "platform": "ios",
            "apps": [
                {
                    "appName": "Yapper",
                    "durationSeconds": 60
                }
            ]
        }))
        .await;

    assert_eq!(response.status_code().as_u16(), 400);
    assert!(
        response.text().contains("must not be in the future"),
        "unexpected future-date rejection: {}",
        response.text()
    );
}
