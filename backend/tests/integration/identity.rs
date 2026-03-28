//! Integration tests for email normalization and linked identity storage.

use super::{
    login_test_session, register_test_session, spawn_test_server_with_pool, test_device_bootstrap,
    TestClient,
};
use axum_test::{TestResponse, TestServer};
use sqlx::PgPool;
use uuid::Uuid;

async fn build_identity_test_server(pool: PgPool) -> Option<(yapper_server::AppState, TestServer)> {
    spawn_test_server_with_pool(pool).await
}

async fn submit_v2_register(
    server: &TestServer,
    email: &str,
    username: &str,
    password: &str,
) -> TestResponse {
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": email,
            "username": username,
            "password": password,
            "device": test_device_bootstrap(&suffix),
        }))
        .await
}

#[sqlx::test(migrations = "./migrations")]
async fn mixed_case_child_email_is_normalized_and_duplicate_rejected(pool: PgPool) {
    let Some((state, server)) = build_identity_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let parent_suffix = format!("parent_{suffix}");
    let parent_session = register_test_session(&server, &parent_suffix).await;
    let parent_client = TestClient::from_session(&server, &parent_session);

    let child_email = format!("MiXeD_{suffix}@Integration.Test");
    let child_username = format!("child_{suffix}");
    let child_password = format!("ChildPass123!{suffix}");
    let child_dob = "2015-03-26";

    let child_resp = parent_client
        .post("/api/v2/parental/children")
        .json(&serde_json::json!({
            "username": child_username,
            "display_name": "Mixed Case Child",
            "email": child_email,
            "password": child_password,
            "date_of_birth": child_dob,
        }))
        .await;

    assert!(
        child_resp.status_code().is_success(),
        "child creation failed: {} - {}",
        child_resp.status_code(),
        child_resp.text()
    );

    let child_body: serde_json::Value = child_resp.json();
    let child_id: Uuid = child_body["child_id"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("missing child_id");

    let stored_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(child_id)
        .fetch_one(state.db.pool())
        .await
        .expect("load child email");
    assert_eq!(stored_email, child_email.to_lowercase());

    let login_body = login_test_session(
        &server,
        &child_email,
        &child_password,
        &format!("child_login_{suffix}"),
    )
    .await;
    assert!(
        !login_body.access_token.is_empty(),
        "mixed-case login should succeed after normalization"
    );

    let duplicate = submit_v2_register(
        &server,
        &child_email.to_uppercase(),
        &format!("child_dup_{suffix}"),
        "DuplicatePass123!",
    )
    .await;
    assert!(
        !duplicate.status_code().is_success(),
        "duplicate registration should fail"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn linked_identities_coexist_and_unlink_individually(pool: PgPool) {
    let Some((state, server)) = build_identity_test_server(pool).await else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = register_test_session(&server, &suffix).await;
    let client = TestClient::from_session(&server, &session);
    let user_id = session.user_id;

    for (provider, subject) in [
        ("discord", format!("discord-{suffix}")),
        ("google", format!("google-{suffix}")),
        ("apple", format!("apple-{suffix}")),
    ] {
        sqlx::query(
            "INSERT INTO user_linked_identities (user_id, provider, provider_subject)
             VALUES ($1, $2, $3)
             ON CONFLICT (user_id, provider) DO UPDATE SET provider_subject = EXCLUDED.provider_subject",
        )
        .bind(user_id)
        .bind(provider)
        .bind(&subject)
        .execute(state.db.pool())
        .await
        .expect("seed linked identity");
    }

    let me = client.get("/api/v2/users/me").await;
    assert!(
        me.status_code().is_success(),
        "GET /users/me failed: {}",
        me.text()
    );
    let me_body: serde_json::Value = me.json();
    assert_eq!(me_body["connections"]["discord"], true);
    assert_eq!(me_body["connections"]["google"], true);
    assert_eq!(me_body["connections"]["apple"], true);

    let unlink = client.delete("/api/v2/users/me/connections/google").await;
    assert_eq!(
        unlink.status_code().as_u16(),
        204,
        "unlink failed: {}",
        unlink.text()
    );

    let me_after = client.get("/api/v2/users/me").await;
    assert!(
        me_after.status_code().is_success(),
        "GET /users/me after unlink failed: {}",
        me_after.text()
    );
    let me_after_body: serde_json::Value = me_after.json();
    assert_eq!(me_after_body["connections"]["discord"], true);
    assert_eq!(me_after_body["connections"]["google"], false);
    assert_eq!(me_after_body["connections"]["apple"], true);

    let linked_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_linked_identities WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(state.db.pool())
            .await
            .expect("count linked identities");
    assert_eq!(linked_count, 2);
}
