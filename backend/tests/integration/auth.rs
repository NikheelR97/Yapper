//! Integration tests - auth endpoints.
//!
//! Tests the full register -> login -> refresh -> logout cycle, plus
//! rate limiting on repeated failed logins and the removed legacy auth gate.
//!
//! Run with:
//!   TEST_DATABASE_URL=postgres://... cargo test --test integration -- auth

use super::{
    authorization_header_name, bearer_header, create_test_user, create_test_user_with_device,
    login_test_session, login_test_user_with_device, register_test_session,
    spawn_test_server_from_pool, spawn_test_server_with_pool, test_device_bootstrap, TestClient,
};
use axum_test::TestServer;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{encode, Algorithm, Header};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::time::{Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use yapper_server::{auth::service::AccessClaims, build_router, AppState};

async fn build_test_server(pool: PgPool) -> Option<(AppState, TestServer)> {
    spawn_test_server_with_pool(pool).await
}

async fn spawn_ws_server(state: AppState) -> Option<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;
    let app = build_router(state);
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    Some(addr)
}

fn mint_access_token(
    state: &AppState,
    user_id: Uuid,
    account_type: &str,
    device_id: Option<Uuid>,
    ttl_secs: i64,
) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        exp: (now + chrono::Duration::seconds(ttl_secs)).timestamp(),
        iat: now.timestamp(),
        kid: state.jwt_keys.kid.clone(),
        account_type: account_type.to_string(),
        device_id,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(state.jwt_keys.kid.clone());
    encode(&header, &claims, &state.jwt_keys.encoding).expect("failed to sign access token")
}

async fn assert_ws_eventually_closed<S>(ws: &mut S)
where
    S: futures::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    for _ in 0..4 {
        let next = tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await
            .expect("timeout waiting for websocket close");
        match next {
            None | Some(Ok(Message::Close(_))) => return,
            Some(Ok(Message::Text(text))) => {
                let body: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(body) => body,
                    Err(_) => continue,
                };
                if body["type"] == "error" {
                    continue;
                }
            }
            Some(Ok(_)) => continue,
            Some(Err(_)) => return,
        }
    }
    panic!("websocket did not close");
}

async fn wait_for_ws_event_type<S>(ws: &mut S, event_type: &str, timeout: Duration) -> serde_json::Value
where
    S: futures::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let deadline = Instant::now() + timeout;
    loop {
        let now = Instant::now();
        let remaining = deadline.saturating_duration_since(now);
        assert!(
            !remaining.is_zero(),
            "timeout waiting for websocket event type {event_type}"
        );

        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("timeout waiting for websocket frame")
            .expect("ws closed before expected frame")
            .expect("ws error");

        if let Message::Text(text) = msg {
            let body: serde_json::Value = serde_json::from_str(&text).expect("valid json");
            if body["type"] == event_type {
                return body;
            }
        }
    }
}

/// Full happy-path cycle: register -> login -> /users/me -> logout.
#[sqlx::test(migrations = "./migrations")]
async fn auth_register_login_me_logout(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("test_{suffix}@integration.test");
    let username = format!("test_{suffix}");
    let password = format!("TestPass123!{suffix}");

    let register = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": &username,
            "password": &password,
            "device": test_device_bootstrap(&suffix),
        }))
        .await;
    assert!(
        register.status_code().is_success(),
        "register failed: {} {}",
        register.status_code(),
        register.text()
    );

    let session = login_test_session(&server, &email, &password, &suffix).await;
    let client = TestClient::from_session(&server, &session);

    let me = client.get("/api/v2/users/me").await;
    assert!(
        me.status_code().is_success(),
        "GET /users/me failed: {}",
        me.text()
    );
    let me_body: serde_json::Value = me.json();
    assert!(me_body["id"].is_string(), "users/me missing id");

    let logout = client.delete("/api/v2/auth/logout").await;
    assert!(
        logout.status_code().is_success(),
        "logout failed: {} {}",
        logout.status_code(),
        logout.text()
    );
}

/// Repeated failed logins (wrong password) must return 401, not 500.
#[sqlx::test(migrations = "./migrations")]
async fn auth_wrong_password_returns_401(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("wrong_pw_{suffix}@integration.test");
    let username = format!("wrong_pw_{suffix}");

    let _reg = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": &username,
            "password": "CorrectPass123!",
            "device": test_device_bootstrap(&suffix),
        }))
        .await;

    for _ in 0..3 {
        let resp = server
            .post("/api/v2/auth/login")
            .json(&serde_json::json!({
                "email": &email,
                "password": "WrongPassword999!",
                "device": test_device_bootstrap(&suffix),
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
#[sqlx::test(migrations = "./migrations")]
async fn auth_duplicate_email_rejected(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("dup_{suffix}@integration.test");

    let reg1 = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": format!("dup1_{suffix}"),
            "password": "TestPass123!",
            "device": test_device_bootstrap(&suffix),
        }))
        .await;
    assert!(
        reg1.status_code().is_success(),
        "first register failed: {}",
        reg1.text()
    );

    let reg2 = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": format!("dup2_{suffix}"),
            "password": "TestPass123!",
            "device": test_device_bootstrap(&suffix),
        }))
        .await;
    let status = reg2.status_code().as_u16();
    assert!(
        status >= 400 && status < 500,
        "duplicate email should return 4xx, got {status}"
    );
}

/// Token refresh: POST /api/v2/auth/refresh returns a new access token.
#[sqlx::test(migrations = "./migrations")]
async fn auth_refresh_returns_new_token(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let session = register_test_session(&server, &suffix).await;
    let refresh_token = session
        .refresh_token
        .as_ref()
        .expect("refresh token should be set in auth session");

    let refresh = server
        .post("/api/v2/auth/refresh")
        .clear_cookies()
        .add_header(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_str(&format!("refresh_token={refresh_token}"))
                .expect("valid refresh cookie header"),
        )
        .json(&serde_json::json!({}))
        .await;

    assert!(
        refresh.status_code().is_success(),
        "refresh failed: {}",
        refresh.text()
    );
    let body: serde_json::Value = refresh.json();
    assert!(
        body["access_token"].is_string(),
        "refresh missing access_token"
    );
}

/// Device-aware auth endpoints must reject legacy v1-style payloads.
#[sqlx::test(migrations = "./migrations")]
async fn legacy_v1_auth_session_endpoints_are_forbidden(pool: PgPool) {
    let Some((_state, server)) = build_test_server(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("legacy_{suffix}@integration.test");
    let username = format!("legacy_{suffix}");

    let register = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": &email,
            "username": &username,
            "password": "TestPass123!",
        }))
        .await;
    assert_eq!(register.status_code().as_u16(), 422);

    let login = server
        .post("/api/v2/auth/login")
        .json(&serde_json::json!({
            "email": &email,
            "password": "TestPass123!",
        }))
        .await;
    assert_eq!(login.status_code().as_u16(), 422);

    let refresh = server.post("/api/v2/auth/refresh").await;
    assert_eq!(refresh.status_code().as_u16(), 401);

    let logout = server.delete("/api/v2/auth/logout").await;
    assert_eq!(logout.status_code().as_u16(), 403);
}

/// REST must reject human access tokens that do not bind a device.
#[sqlx::test(migrations = "./migrations")]
async fn rest_rejects_device_less_human_tokens(pool: PgPool) {
    let Some((state, server)) = build_test_server(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, _, _) = create_test_user(&server, &suffix).await;
    let token = mint_access_token(&state, user_id, "standard", None, 4);

    let resp = server
        .get("/api/v2/devices")
        .add_header(authorization_header_name(), bearer_header(&token))
        .await;

    assert_eq!(
        resp.status_code().as_u16(),
        401,
        "device-less human token should be rejected"
    );
}

/// WebSocket auth must reject human access tokens without a device binding.
#[sqlx::test(migrations = "./migrations")]
async fn ws_rejects_device_less_human_tokens(pool: PgPool) {
    let Some((state, server)) = build_test_server(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, _, _) = create_test_user(&server, &suffix).await;
    let ws_addr = spawn_ws_server(state.clone()).await.expect("ws server");
    let url = format!("ws://{ws_addr}/ws");
    let (mut ws, _) = connect_async(url).await.expect("ws connect");
    let token = mint_access_token(&state, user_id, "standard", None, 4);

    ws.send(Message::Text(
        serde_json::json!({ "type": "auth", "token": token }).to_string(),
    ))
    .await
    .expect("send auth");
    assert_ws_eventually_closed(&mut ws).await;
}

/// Reauth must use the same user and same device, and it must refresh the live session.
#[sqlx::test(migrations = "./migrations")]
async fn ws_reauth_refreshes_same_user_same_device_session(pool: PgPool) {
    let Some((state, server)) = build_test_server(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, _, _, device_id) = create_test_user_with_device(&server, &suffix).await;
    let ws_addr = spawn_ws_server(state.clone()).await.expect("ws server");
    let url = format!("ws://{ws_addr}/ws");
    let (mut ws, _) = connect_async(url).await.expect("ws connect");
    let short_token = mint_access_token(&state, user_id, "standard", Some(device_id), 4);

    ws.send(Message::Text(
        serde_json::json!({ "type": "auth", "token": short_token }).to_string(),
    ))
    .await
    .expect("send auth");

    let reauth_prompt =
        wait_for_ws_event_type(&mut ws, "re_auth_required", Duration::from_secs(12)).await;
    assert_eq!(reauth_prompt["type"], "re_auth_required");

    let fresh_reauth_token = mint_access_token(&state, user_id, "standard", Some(device_id), 300);

    ws.send(Message::Text(
        serde_json::json!({ "type": "reauth", "token": fresh_reauth_token }).to_string(),
    ))
    .await
    .expect("send reauth");

    ws.send(Message::Text(
        serde_json::json!({ "type": "ping" }).to_string(),
    ))
    .await
    .expect("send ping after reauth");
    let pong = tokio::time::timeout(Duration::from_secs(3), ws.next())
        .await
        .expect("timeout waiting for pong")
        .expect("ws closed after reauth")
        .expect("ws error after reauth");
    let pong_text = match pong {
        Message::Text(text) => text,
        other => panic!("expected pong text frame, got {other:?}"),
    };
    let pong_body: serde_json::Value = serde_json::from_str(&pong_text).expect("valid pong json");
    assert_eq!(
        pong_body["type"],
        "pong",
        "expected pong after reauth, got frame: {pong_body}"
    );
}

/// Reauth with a different device must close the connection.
#[sqlx::test(migrations = "./migrations")]
async fn ws_reauth_rejects_different_device(pool: PgPool) {
    let Some((state, server)) = build_test_server(pool).await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, _, _, device_id) = create_test_user_with_device(&server, &suffix).await;
    let other_suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("test_{suffix}@integration.test");
    let password = format!("TestPass123!{suffix}");
    let (same_user_id, _, _, wrong_device_id) =
        login_test_user_with_device(&server, &email, &password, &other_suffix).await;

    assert_eq!(user_id, same_user_id);
    assert_ne!(device_id, wrong_device_id);

    let ws_addr = spawn_ws_server(state.clone()).await.expect("ws server");
    let url = format!("ws://{ws_addr}/ws");
    let (mut ws, _) = connect_async(url).await.expect("ws connect");
    let short_token = mint_access_token(&state, user_id, "standard", Some(device_id), 4);

    ws.send(Message::Text(
        serde_json::json!({ "type": "auth", "token": short_token }).to_string(),
    ))
    .await
    .expect("send auth");

    let reauth_prompt =
        wait_for_ws_event_type(&mut ws, "re_auth_required", Duration::from_secs(12)).await;
    assert_eq!(reauth_prompt["type"], "re_auth_required");

    let wrong_reauth_token =
        mint_access_token(&state, same_user_id, "standard", Some(wrong_device_id), 300);

    ws.send(Message::Text(
        serde_json::json!({ "type": "reauth", "token": wrong_reauth_token }).to_string(),
    ))
    .await
    .expect("send wrong reauth");
    assert_ws_eventually_closed(&mut ws).await;
}
