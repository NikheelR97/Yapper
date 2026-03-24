//! Integration test helpers.
//!
//! Shared utilities for building a test `AppState` backed by a real PostgreSQL
//! database (TEST_DATABASE_URL), registering users, and making authenticated
//! requests via `axum_test::TestServer`.

use axum::http::{header::AUTHORIZATION, HeaderName, HeaderValue};
use axum_test::TestServer;
use governor::{Quota, RateLimiter};
use serde_json::Value;
use std::{collections::HashSet, num::NonZeroU32, sync::Arc};
use uuid::Uuid;
use yapper_server::{
    auth::{JwtKeys, LoginRateLimiter, OAuthStateStore},
    build_router,
    db::Database,
    hub::Hub,
    AppState, DiscordImportStateStore, IpRateLimiter,
};

/// Build a `TestServer` against a real PostgreSQL test database.
///
/// Reads `TEST_DATABASE_URL` from the environment (or falls back to
/// `DATABASE_URL`). Each test should use an isolated schema or run with
/// `serial_test::serial` to prevent race conditions.
pub async fn build_test_state() -> Option<AppState> {
    let db_url = match std::env::var("TEST_DATABASE_URL").or_else(|_| std::env::var("DATABASE_URL"))
    {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Skipping integration tests: TEST_DATABASE_URL or DATABASE_URL is not set");
            return None;
        }
    };

    let db = Database::connect(&db_url)
        .await
        .expect("Failed to connect to test database");
    db.run_migrations().await.expect("Failed to run migrations");

    let hub = Arc::new(Hub::new());
    let quota = Quota::per_minute(NonZeroU32::new(10_000).unwrap())
        .allow_burst(NonZeroU32::new(5_000).unwrap());
    let rate_limiter: IpRateLimiter = Arc::new(RateLimiter::keyed(quota));
    let trusted_proxy_ips = Arc::new(HashSet::new());

    // JWT keys: read from env (JWT_PRIVATE_KEY / JWT_PRIVATE_KEY_PATH).
    // Integration tests require these to be set in the test environment.
    let jwt_keys = match JwtKeys::from_env() {
        Ok(keys) => Arc::new(keys),
        Err(error) => {
            eprintln!("Skipping integration tests: {error}");
            return None;
        }
    };

    Some(AppState {
        db,
        hub,
        rate_limiter,
        trusted_proxy_ips,
        jwt_keys,
        login_limiter: Arc::new(LoginRateLimiter::new()),
        oauth_states: Arc::new(OAuthStateStore::new()),
        discord_import_states: Arc::new(DiscordImportStateStore::new()),
        http_client: reqwest::Client::new(),
    })
}

pub async fn spawn_test_server() -> Option<TestServer> {
    let state = build_test_state().await?;
    Some(TestServer::new(build_router(state)).expect("Failed to create test server"))
}

/// Register a new user and return their (user_id, access_token, csrf_token).
pub async fn create_test_user(server: &TestServer, suffix: &str) -> (Uuid, String, String) {
    let email = format!("test_{suffix}@integration.test");
    let username = format!("test_{suffix}");
    let password = format!("TestPass123!{suffix}");

    let resp = server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({
            "email": email,
            "username": username,
            "password": password,
        }))
        .await;

    // Accept 201 (created) or 200 (already exists — idempotent for test setup)
    assert!(
        resp.status_code().is_success(),
        "register failed: {} — {:?}",
        resp.status_code(),
        resp.text()
    );

    login_test_user(server, &email, &password).await
}

/// Log in an existing user and return (user_id, access_token, csrf_token).
pub async fn login_test_user(
    server: &TestServer,
    email: &str,
    password: &str,
) -> (Uuid, String, String) {
    let resp = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({
            "email": email,
            "password": password,
        }))
        .await;

    assert!(
        resp.status_code().is_success(),
        "login failed: {} — {:?}",
        resp.status_code(),
        resp.text()
    );

    let body: Value = resp.json();
    let access_token = body["access_token"]
        .as_str()
        .expect("missing access_token")
        .to_string();
    let csrf_token = body["csrf_token"]
        .as_str()
        .expect("missing csrf_token")
        .to_string();
    let user_id: Uuid = body["user"]["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("missing or invalid user.id");

    (user_id, access_token, csrf_token)
}

pub fn bearer_header(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("Bearer {token}")).expect("valid bearer header")
}

pub fn csrf_header_name() -> HeaderName {
    HeaderName::from_static("x-csrf-token")
}

pub fn csrf_header_value(token: &str) -> HeaderValue {
    HeaderValue::from_str(token).expect("valid csrf header")
}

pub fn authorization_header_name() -> HeaderName {
    AUTHORIZATION
}

pub mod account;
pub mod auth;
pub mod devices;
pub mod keys;
