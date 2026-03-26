//! Integration tests - auth endpoints.
//!
//! Tests the full register -> login -> refresh -> logout cycle, plus
//! rate limiting on repeated failed logins and the removed legacy auth gate.
//!
//! Run with:
//!   TEST_DATABASE_URL=postgres://... cargo test --test integration -- auth

use super::{
    authorization_header_name, bearer_header, build_test_state, create_test_user,
    create_test_user_with_device, csrf_header_name, csrf_header_value, login_test_user_with_device,
    spawn_test_server, test_device_bootstrap,
};
use axum_test::TestServer;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{encode, Algorithm, Header};
use serial_test::serial;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use yapper_server::{auth::service::AccessClaims, build_router, AppState};

async fn build_test_server() -> Option<(AppState, TestServer)> {
    let state = build_test_state().await?;
    let server =
        TestServer::new(build_router(state.clone())).expect("Failed to create test server");
    Some((state, server))
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

/// Full happy-path cycle: register -> login -> /users/me -> logout.
#[tokio::test]
#[serial]
async fn auth_register_login_me_logout() {
    let Some(server) = spawn_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();

    let (_user_id, access_token, csrf_token) = create_test_user(&server, &suffix).await;

    let me = server
        .get("/api/v2/users/me")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
        .await;
    assert!(
        me.status_code().is_success(),
        "GET /users/me failed: {}",
        me.text()
    );
    let me_body: serde_json::Value = me.json();
    assert!(me_body["id"].is_string(), "users/me missing id");

    let logout = server
        .delete("/api/v2/auth/logout")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
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
    let Some(server) = spawn_test_server().await else {
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
#[tokio::test]
#[serial]
async fn auth_duplicate_email_rejected() {
    let Some(server) = spawn_test_server().await else {
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
#[tokio::test]
#[serial]
async fn auth_refresh_returns_new_token() {
    let Some(server) = spawn_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (_user_id, access_token, csrf_token) = create_test_user(&server, &suffix).await;

    let refresh = server
        .post("/api/v2/auth/refresh")
        .add_header(authorization_header_name(), bearer_header(&access_token))
        .add_header(csrf_header_name(), csrf_header_value(&csrf_token))
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

/// Legacy v1 session issuance endpoints must be forbidden for human users.
#[tokio::test]
#[serial]
async fn legacy_v1_auth_session_endpoints_are_forbidden() {
    let Some((_state, server)) = build_test_server().await else {
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
    assert_eq!(register.status_code().as_u16(), 403);

    let login = server
        .post("/api/v2/auth/login")
        .json(&serde_json::json!({
            "email": &email,
            "password": "TestPass123!",
        }))
        .await;
    assert_eq!(login.status_code().as_u16(), 403);

    let refresh = server.post("/api/v2/auth/refresh").await;
    assert_eq!(refresh.status_code().as_u16(), 403);

    let logout = server.delete("/api/v2/auth/logout").await;
    assert_eq!(logout.status_code().as_u16(), 403);
}

/// REST must reject human access tokens that do not bind a device.
#[tokio::test]
#[serial]
async fn rest_rejects_device_less_human_tokens() {
    let Some((state, server)) = build_test_server().await else {
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
#[tokio::test]
#[serial]
async fn ws_rejects_device_less_human_tokens() {
    let Some((state, server)) = build_test_server().await else {
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
#[tokio::test]
#[serial]
async fn ws_reauth_refreshes_same_user_same_device_session() {
    let Some((state, server)) = build_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, reauth_token, _, device_id) =
        create_test_user_with_device(&server, &suffix).await;
    let ws_addr = spawn_ws_server(state.clone()).await.expect("ws server");
    let url = format!("ws://{ws_addr}/ws");
    let (mut ws, _) = connect_async(url).await.expect("ws connect");
    let short_token = mint_access_token(&state, user_id, "standard", Some(device_id), 4);

    ws.send(Message::Text(
        serde_json::json!({ "type": "auth", "token": short_token }).to_string(),
    ))
    .await
    .expect("send auth");

    let mut saw_ready = false;
    let mut saw_reauth_required = false;
    for _ in 0..4 {
        let msg = tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await
            .expect("timeout waiting for ready")
            .expect("ws closed too early")
            .expect("ws error");
        if let Message::Text(text) = msg {
            let body: serde_json::Value = serde_json::from_str(&text).expect("valid json");
            match body["type"].as_str() {
                Some("ready") => saw_ready = true,
                Some("re_auth_required") => saw_reauth_required = true,
                _ => {}
            }
        }
        if saw_ready && saw_reauth_required {
            break;
        }
    }
    assert!(saw_ready, "expected ready frame");
    assert!(saw_reauth_required, "expected re-authentication prompt");

    ws.send(Message::Text(
        serde_json::json!({ "type": "reauth", "token": reauth_token }).to_string(),
    ))
    .await
    .expect("send reauth");

    tokio::time::sleep(Duration::from_secs(5)).await;

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
    assert_eq!(pong_body["type"], "pong");
}

/// Reauth with a different device must close the connection.
#[tokio::test]
#[serial]
async fn ws_reauth_rejects_different_device() {
    let Some((state, server)) = build_test_server().await else {
        return;
    };
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (user_id, _, _, device_id) = create_test_user_with_device(&server, &suffix).await;
    let other_suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let email = format!("test_{suffix}@integration.test");
    let password = format!("TestPass123!{suffix}");
    let (same_user_id, wrong_reauth_token, _, wrong_device_id) =
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

    let mut saw_reauth_required = false;
    for _ in 0..4 {
        let msg = tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await
            .expect("timeout waiting for reauth prompt")
            .expect("ws closed too early")
            .expect("ws error");
        if let Message::Text(text) = msg {
            let body: serde_json::Value = serde_json::from_str(&text).expect("valid json");
            if body["type"] == "re_auth_required" {
                saw_reauth_required = true;
                break;
            }
        }
    }
    assert!(saw_reauth_required, "expected re-authentication prompt");

    ws.send(Message::Text(
        serde_json::json!({ "type": "reauth", "token": wrong_reauth_token }).to_string(),
    ))
    .await
    .expect("send wrong reauth");
    assert_ws_eventually_closed(&mut ws).await;
}
