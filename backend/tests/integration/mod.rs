//! Integration test helpers.
//!
//! Shared utilities for building a test `AppState` backed by a real PostgreSQL
//! database (TEST_DATABASE_URL), registering users, and making authenticated
//! requests via `axum_test::TestServer`.

use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use axum_test::{TestRequest, TestResponse, TestServer, TestServerConfig};
use governor::{Quota, RateLimiter};
use sqlx::PgPool;
use serde_json::Value;
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
    net::SocketAddr,
    num::NonZeroU32,
    sync::Arc,
};
use uuid::Uuid;
use yapper_server::{
    auth::{JwtKeys, LoginRateLimiter, OAuthStateStore},
    build_router,
    db::Database,
    hub::Hub,
    AppState, DiscordImportStateStore, IpRateLimiter,
};

#[derive(Clone, Debug)]
pub struct AuthSession {
    pub user_id: Uuid,
    pub access_token: String,
    pub csrf_token: String,
    pub refresh_token: Option<String>,
    pub device_id: Option<Uuid>,
}

pub struct TestClient<'a> {
    server: &'a TestServer,
    access_token: String,
    csrf_token: String,
    refresh_token: Option<String>,
}

pub fn build_test_state_from_db(db: Database) -> Option<AppState> {
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

pub async fn build_test_state_from_pool(pool: PgPool) -> Option<AppState> {
    build_test_state_from_db(Database::from_pool(pool))
}

pub async fn spawn_test_server_from_pool(pool: PgPool) -> Option<TestServer> {
    let state = build_test_state_from_pool(pool).await?;
    let config = TestServerConfig::builder().save_cookies().build();
    Some(
        TestServer::new_with_config(
            build_router(state).into_make_service_with_connect_info::<SocketAddr>(),
            config,
        )
            .expect("Failed to create test server"),
    )
}

pub async fn spawn_test_server_with_pool(pool: PgPool) -> Option<(AppState, TestServer)> {
    let state = build_test_state_from_pool(pool).await?;
    let server = spawn_test_server_from_state(state.clone());
    Some((state, server))
}

pub fn spawn_test_server_from_state(state: AppState) -> TestServer {
    let config = TestServerConfig::builder().save_cookies().build();
    TestServer::new_with_config(
        build_router(state).into_make_service_with_connect_info::<SocketAddr>(),
        config,
    )
    .expect("Failed to create test server")
}

pub fn test_device_bootstrap(suffix: &str) -> serde_json::Value {
    let mut hasher = DefaultHasher::new();
    suffix.hash(&mut hasher);
    let hi = hasher.finish();
    let lo = hi.rotate_left(17) ^ 0xa5a5_5a5a_dead_beef;
    let installation_id = format!(
        "{:08x}-{:04x}-4{:03x}-a{:03x}-{:012x}",
        (hi >> 32) as u32,
        ((hi >> 16) & 0xffff) as u16,
        (hi & 0x0fff) as u16,
        ((lo >> 48) & 0x0fff) as u16,
        lo & 0x0000_ffff_ffff_ffff,
    );
    serde_json::json!({
        "installation_id": installation_id,
        "platform": "web",
        "label": format!("Integration Test Browser {suffix}"),
    })
}

fn extract_cookie_value(headers: &HeaderMap<HeaderValue>, cookie_name: &str) -> Option<String> {
    let prefix = format!("{cookie_name}=");
    headers
        .get_all(axum::http::header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|raw| {
            raw.strip_prefix(&prefix)
                .and_then(|rest| rest.split(';').next())
                .map(str::to_string)
        })
}

fn parse_auth_session(response: &TestResponse) -> AuthSession {
    let body: Value = response.json();
    AuthSession {
        access_token: body["access_token"]
            .as_str()
            .expect("missing access_token")
            .to_string(),
        csrf_token: body["csrf_token"]
            .as_str()
            .expect("missing csrf_token")
            .to_string(),
        refresh_token: extract_cookie_value(response.headers(), "refresh_token"),
        user_id: body["user"]["id"]
            .as_str()
            .and_then(|value| value.parse().ok())
            .expect("missing or invalid user.id"),
        device_id: body["device"]["id"]
            .as_str()
            .and_then(|value| value.parse().ok()),
    }
}

impl<'a> TestClient<'a> {
    pub fn from_session(server: &'a TestServer, session: &AuthSession) -> Self {
        Self {
            server,
            access_token: session.access_token.clone(),
            csrf_token: session.csrf_token.clone(),
            refresh_token: session.refresh_token.clone(),
        }
    }

    fn cookie_header_value(&self) -> HeaderValue {
        let mut cookies = Vec::new();
        if let Some(refresh_token) = &self.refresh_token {
            cookies.push(format!("refresh_token={refresh_token}"));
        }
        cookies.push(format!("csrf_token={}", self.csrf_token));
        HeaderValue::from_str(&cookies.join("; ")).expect("valid cookie header")
    }

    fn authenticated_request(&self, request: TestRequest) -> TestRequest {
        request.add_header(authorization_header_name(), bearer_header(&self.access_token))
    }

    fn mutating_request(&self, request: TestRequest) -> TestRequest {
        self.authenticated_request(request.clear_cookies())
            .add_header(csrf_header_name(), csrf_header_value(&self.csrf_token))
            .add_header(axum::http::header::COOKIE, self.cookie_header_value())
    }

    pub fn get(&self, path: &str) -> TestRequest {
        self.authenticated_request(self.server.get(path))
    }

    pub fn post(&self, path: &str) -> TestRequest {
        self.mutating_request(self.server.post(path))
    }

    pub fn patch(&self, path: &str) -> TestRequest {
        self.mutating_request(self.server.patch(path))
    }

    #[allow(dead_code)]
    pub fn put(&self, path: &str) -> TestRequest {
        self.mutating_request(self.server.put(path))
    }

    pub fn delete(&self, path: &str) -> TestRequest {
        self.mutating_request(self.server.delete(path))
    }
}

pub async fn register_test_session(server: &TestServer, suffix: &str) -> AuthSession {
    let email = format!("test_{suffix}@integration.test");
    let username = format!("test_{suffix}");
    let password = format!("TestPass123!{suffix}");

    let response = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": email,
            "username": username,
            "password": password,
            "device": test_device_bootstrap(suffix),
        }))
        .await;

    assert!(
        response.status_code().is_success(),
        "register failed: {} - {}",
        response.status_code(),
        response.text()
    );

    parse_auth_session(&response)
}

pub async fn login_test_session(
    server: &TestServer,
    email: &str,
    password: &str,
    device_suffix: &str,
) -> AuthSession {
    let response = server
        .post("/api/v2/auth/login")
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "device": test_device_bootstrap(device_suffix),
        }))
        .await;

    assert!(
        response.status_code().is_success(),
        "login failed: {} - {}",
        response.status_code(),
        response.text()
    );

    parse_auth_session(&response)
}

/// Register a new user and return their (user_id, access_token, csrf_token).
#[allow(unreachable_code)]
pub async fn create_test_user(server: &TestServer, suffix: &str) -> (Uuid, String, String) {
    let session = register_test_session(server, suffix).await;
    return (session.user_id, session.access_token, session.csrf_token);

    let email = format!("test_{suffix}@integration.test");
    let username = format!("test_{suffix}");
    let password = format!("TestPass123!{suffix}");

    let resp = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": email,
            "username": username,
            "password": password,
            "device": test_device_bootstrap(suffix),
        }))
        .await;

    // Accept 201 (created) or 200 (already exists — idempotent for test setup)
    assert!(
        resp.status_code().is_success(),
        "register failed: {} — {:?}",
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

/// Register a new user and return their (user_id, access_token, csrf_token, device_id).
#[allow(unreachable_code)]
pub async fn create_test_user_with_device(
    server: &TestServer,
    suffix: &str,
) -> (Uuid, String, String, Uuid) {
    let session = register_test_session(server, suffix).await;
    return (
        session.user_id,
        session.access_token,
        session.csrf_token,
        session.device_id.expect("missing device.id"),
    );

    let email = format!("test_{suffix}@integration.test");
    let username = format!("test_{suffix}");
    let password = format!("TestPass123!{suffix}");

    let resp = server
        .post("/api/v2/auth/register")
        .json(&serde_json::json!({
            "email": email,
            "username": username,
            "password": password,
            "device": test_device_bootstrap(suffix),
        }))
        .await;

    assert!(
        resp.status_code().is_success(),
        "register failed: {} â€” {:?}",
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
    let device_id: Uuid = body["device"]["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("missing or invalid device.id");

    (user_id, access_token, csrf_token, device_id)
}

/// Log in an existing user and return (user_id, access_token, csrf_token).
#[allow(dead_code, unreachable_code)]
pub async fn login_test_user(
    server: &TestServer,
    email: &str,
    password: &str,
) -> (Uuid, String, String) {
    let device_suffix = email
        .strip_prefix("test_")
        .and_then(|rest| rest.split('@').next())
        .expect("test email must use test_<suffix>@integration.test");
    let session = login_test_session(server, email, password, device_suffix).await;
    return (session.user_id, session.access_token, session.csrf_token);

    let suffix = email
        .strip_prefix("test_")
        .and_then(|rest| rest.split('@').next())
        .expect("test email must use test_<suffix>@integration.test");
    let resp = server
        .post("/api/v2/auth/login")
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "device": test_device_bootstrap(suffix),
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

/// Log in an existing user and return the active device ID too.
#[allow(unreachable_code)]
pub async fn login_test_user_with_device(
    server: &TestServer,
    email: &str,
    password: &str,
    device_suffix: &str,
) -> (Uuid, String, String, Uuid) {
    let session = login_test_session(server, email, password, device_suffix).await;
    return (
        session.user_id,
        session.access_token,
        session.csrf_token,
        session.device_id.expect("missing device.id"),
    );

    let resp = server
        .post("/api/v2/auth/login")
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "device": test_device_bootstrap(device_suffix),
        }))
        .await;

    assert!(
        resp.status_code().is_success(),
        "login failed: {} â€” {:?}",
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
    let device_id: Uuid = body["device"]["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("missing or invalid device.id");

    (user_id, access_token, csrf_token, device_id)
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

#[allow(dead_code)]
pub fn csrf_cookie_header_name() -> HeaderName {
    axum::http::header::COOKIE
}

#[allow(dead_code)]
pub fn csrf_cookie_header_value(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("csrf_token={token}")).expect("valid csrf cookie header")
}

pub fn authorization_header_name() -> HeaderName {
    AUTHORIZATION
}

pub mod account;
pub mod auth;
pub mod devices;
pub mod identity;
pub mod keys;
pub mod messages;
pub mod canvas;
pub mod parental;
pub mod privacy;
pub mod screentime;
pub mod server_caps;
pub mod v1_absence;
