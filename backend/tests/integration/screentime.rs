//! Integration tests for screentime authorization.

use super::{
    authorization_header_name, bearer_header, create_test_user_with_device, csrf_header_name,
    csrf_header_value, spawn_test_server_with_pool,
};
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
    let (_, parent_a_access, parent_a_csrf, _) =
        create_test_user_with_device(&server, &format!("screen_parent_a_{suffix}")).await;
    let (_, parent_b_access, _, _) =
        create_test_user_with_device(&server, &format!("screen_parent_b_{suffix}")).await;

    let child_resp = server
        .post("/api/v2/parental/children")
        .add_header(authorization_header_name(), bearer_header(&parent_a_access))
        .add_header(csrf_header_name(), csrf_header_value(&parent_a_csrf))
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

    let forbidden = server
        .get(&format!("/api/v2/parental/children/{child_id}/screentime?period=today"))
        .add_header(authorization_header_name(), bearer_header(&parent_b_access))
        .await;
    assert_eq!(forbidden.status_code().as_u16(), 403, "wrong parent should be rejected");

    let allowed = server
        .get(&format!("/api/v2/parental/children/{child_id}/screentime?period=today"))
        .add_header(authorization_header_name(), bearer_header(&parent_a_access))
        .await;
    assert!(allowed.status_code().is_success(), "linked parent should be allowed: {}", allowed.text());
}

#[sqlx::test(migrations = "./migrations")]
async fn screentime_reports_reject_future_recorded_dates(pool: PgPool) {
    let Some((_state, server)) = build_screentime_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_user_id, access_token, csrf_token, _device_id) =
        create_test_user_with_device(&server, &format!("screen_future_{suffix}")).await;
    let future_date = (chrono::Utc::now().date_naive() + chrono::Duration::days(1)).to_string();

    let response = server
        .post("/api/v2/screentime/report")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
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
