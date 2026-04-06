//! Yapper server library crate.
//!
//! Exports `AppState` and `build_router` so integration tests in `tests/`
//! can construct the full application without starting a real TCP listener.

#![deny(warnings)]

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, Method, StatusCode},
    middleware::Next,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, RateLimiter};
use serde_json::json;
use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    num::NonZeroU32,
    sync::Arc,
    time::Duration,
};
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

const NOSNIFF: &str = "nosniff";
const DENY_FRAME: &str = "DENY";
const HSTS: &str = "max-age=63072000; includeSubDomains; preload";
const CSP_API: &str = "default-src 'none'; frame-ancestors 'none'";

pub mod auth;
pub mod bots;
pub mod canvas;
pub mod channels;
pub mod constants;
pub mod csrf;
pub mod db;
pub mod devices;
pub mod discord;
pub mod emojis;
pub mod error;
pub mod explore;
pub mod hub;
pub mod keys;
pub mod media;
pub mod messages;
pub mod notifications;
pub mod parental;
pub mod premium;
pub mod retention;
pub mod screentime;
pub mod servers;
pub mod support;
pub mod users;

use auth::{JwtKeys, LoginRateLimiter, OAuthStateStore};
use db::Database;
use hub::Hub;

/// Encode a byte slice as lowercase hexadecimal.
/// Writing to a pre-allocated `String` is infallible, so this cannot panic.
pub fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

pub type IpRateLimiter = Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

pub type DiscordImportStateStore = dashmap::DashMap<String, (uuid::Uuid, std::time::Instant)>;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub hub: Arc<Hub>,
    pub rate_limiter: IpRateLimiter,
    pub trusted_proxy_ips: Arc<HashSet<IpAddr>>,
    pub jwt_keys: Arc<JwtKeys>,
    pub login_limiter: Arc<LoginRateLimiter>,
    pub oauth_states: Arc<OAuthStateStore>,
    pub discord_import_states: Arc<DiscordImportStateStore>,
    pub http_client: reqwest::Client,
}

pub fn env_non_zero_u32(key: &str, default: u32) -> NonZeroU32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .and_then(NonZeroU32::new)
        .or_else(|| NonZeroU32::new(default))
        .expect("default must be non-zero")
}

pub fn load_trusted_proxy_ips() -> HashSet<IpAddr> {
    std::env::var("TRUSTED_PROXY_IPS")
        .ok()
        .map(|raw| {
            raw.split(',')
                .filter_map(|ip| ip.trim().parse::<IpAddr>().ok())
                .collect::<HashSet<_>>()
        })
        .filter(|set| !set.is_empty())
        .unwrap_or_else(|| {
            [
                IpAddr::from([127, 0, 0, 1]),
                IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]),
            ]
            .into_iter()
            .collect()
        })
}

/// Build the full application router, ready to be served or used by integration tests.
pub fn build_router(state: AppState) -> Router {
    let api_v2 = api_router_v2().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        api_rate_limit_check,
    ));

    Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(hub::ws_handler))
        .nest("/auth/oauth", auth::oauth_router())
        .nest("/api/v2", api_v2)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static(NOSNIFF),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static(DENY_FRAME),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static(HSTS),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP_API),
        ))
        .layer(cors_layer())
        .with_state(state)
}

pub(crate) fn api_router_v2() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::v2_router())
        .nest("/users", users::router())
        .nest("/account", users::account_router())
        .nest("/servers", servers::router())
        .nest("/channels", channels::router())
        .nest("/conversations", messages::v2_router())
        .nest("/keys", keys::v2_router())
        .nest("/media", media::router())
        .merge(canvas::router())
        .merge(explore::router())
        .nest("/emojis", emojis::router())
        .nest("/parental", parental::router())
        .merge(screentime::router())
        .nest("/bots", bots::router())
        .nest("/discord", discord::router())
        .nest("/premium", premium::router())
        .nest("/notifications", notifications::router())
        .nest("/support", support::router())
        .nest("/devices", devices::router())
        .layer(axum::middleware::from_fn(csrf::csrf_check))
}

pub(crate) fn cors_layer() -> CorsLayer {
    use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
    use axum::http::HeaderName;

    let origins: Vec<HeaderValue> = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| {
            "http://localhost:5173,tauri://localhost,capacitor://localhost,http://tauri.localhost"
                .to_string()
        })
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            CONTENT_TYPE,
            AUTHORIZATION,
            ACCEPT,
            HeaderName::from_static("x-csrf-token"),
            HeaderName::from_static("x-refresh-token"),
        ])
        .allow_credentials(true)
}

/// Liveness probe with database verification.
///
/// Returns `200 { "ok": true, "db": "ok" }` only if a `SELECT 1` against the
/// Neon pool completes within 2 seconds. Otherwise returns `503` so Fly.io's
/// HTTP probe can distinguish a healthy VM from one whose database connection
/// has collapsed (Neon auto-suspend, pool exhaustion, DNS flap).
///
/// Audit ref: MED-001 — prior to this change `/health` returned 200
/// unconditionally, which gave Fly.io and external monitors a false-clean
/// signal during DB outages.
async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(state.db.pool()),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false);

    if db_ok {
        (StatusCode::OK, Json(json!({ "ok": true, "db": "ok" })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "ok": false, "db": "down" })),
        )
    }
}

async fn api_rate_limit_check(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    if state.rate_limiter.check_key(&addr.ip()).is_err() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(req).await)
}
