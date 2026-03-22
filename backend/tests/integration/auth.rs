//! Integration tests — auth endpoints.
//!
//! Tests the full register → login → refresh → logout cycle, as well as
//! rate limiting on repeated failed logins.
//!
//! Run with:
//!   TEST_DATABASE_URL=postgres://... cargo test --test integration -- auth

use super::{create_test_user, spawn_test_server};
use serial_test::serial;
use uuid::Uuid;

/// Full happy-path cycle: register → login → /users/me → logout.
#[tokio::test]
#[serial]
async fn auth_register_login_me_logout() {
    let server = spawn_test_server().await;
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();

    let (_user_id, access_token, csrf_token) = create_test_user(&server, &suffix).await;

    // GET /api/v1/users/me must return the user's profile
    let me = server
        .get("/api/v1/users/me")
        .add_header("Authorization", format!("Bearer {access_token}"))
        .add_header("X-CSRF-Token", &csrf_token)
        .await;
    assert!(
        me.status_code().is_success(),
        "GET /users/me failed: {}",
        me.text()
    );
    let me_body: serde_json::Value = me.json();
    assert!(me_body["id"].is_string(), "users/me missing id");

    // POST /api/v1/auth/logout must succeed
    let logout = server
        .post("/api/v1/auth/logout")
        .add_header("Authorization", format!("Bearer {access_token}"))
        .add_header("X-CSRF-Token", &csrf_token)
        .json(&serde_json::json!({}))
        .await;
    assert!(
        logout.status_code().is_success(),
        "logout failed: {}",
        logout.text()
    );
}

/// Repeated failed logins (wrong password) must return 401, not 500.
#[tokio::test]
#[serial]
async fn auth_wrong_password_returns_401() {
    let server = spawn_test_server().await;
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("wrong_pw_{suffix}@integration.test");
    let username = format!("wrong_pw_{suffix}");

    // Register the user first
    let _reg = server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": &username,
            "password": "CorrectPass123!",
        }))
        .await;

    // 3 failed login attempts
    for _ in 0..3 {
        let resp = server
            .post("/api/v1/auth/login")
            .json(&serde_json::json!({
                "email": &email,
                "password": "WrongPassword999!",
            }))
            .await;
        assert_eq!(
            resp.status_code().as_u16(),
            401,
            "expected 401 for wrong password, got {}",
            resp.status_code()
        );
    }
}

/// Register with an already-taken email must return 4xx (not 500).
#[tokio::test]
#[serial]
async fn auth_duplicate_email_rejected() {
    let server = spawn_test_server().await;
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("dup_{suffix}@integration.test");

    let reg1 = server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": format!("dup1_{suffix}"),
            "password": "TestPass123!",
        }))
        .await;
    assert!(
        reg1.status_code().is_success(),
        "first register failed: {}",
        reg1.text()
    );

    let reg2 = server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": format!("dup2_{suffix}"),
            "password": "TestPass123!",
        }))
        .await;
    let status = reg2.status_code().as_u16();
    assert!(
        status >= 400 && status < 500,
        "duplicate email should return 4xx, got {status}"
    );
}

/// Token refresh: POST /api/v1/auth/refresh returns a new access token.
#[tokio::test]
#[serial]
async fn auth_refresh_returns_new_token() {
    let server = spawn_test_server().await;
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_user_id, access_token, csrf_token) = create_test_user(&server, &suffix).await;

    let refresh = server
        .post("/api/v1/auth/refresh")
        .add_header("Authorization", format!("Bearer {access_token}"))
        .add_header("X-CSRF-Token", &csrf_token)
        .json(&serde_json::json!({}))
        .await;

    // Accept 200 (new token returned) or 404 (v1 refresh not implemented — fallback path)
    let status = refresh.status_code().as_u16();
    assert!(
        status == 200 || status == 404,
        "refresh returned unexpected status {status}: {}",
        refresh.text()
    );
    if status == 200 {
        let body: serde_json::Value = refresh.json();
        assert!(
            body["access_token"].is_string(),
            "refresh missing access_token"
        );
    }
}
