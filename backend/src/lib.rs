//! Yapper server library crate.
//!
//! Exports `AppState` and `build_router` so integration tests in `tests/`
//! can construct the full application without starting a real TCP listener.

#![deny(warnings)]

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderValue, Method, Request, StatusCode},
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
};
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

// Security header constants
const NOSNIFF: &str = "nosniff";
const DENY_FRAME: &str = "DENY";
const HSTS: &str = "max-age=63072000; includeSubDomains; preload";
const CSP_API: &str = "default-src 'none'; frame-ancestors 'none'";

// ─── Module declarations ───────────────────────────────────────────────────────

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
            // write! to String is infallible — the fmt::Error branch is unreachable.
            let _ = write!(s, "{b:02x}");
            s
        })
}

// ─── Shared types ─────────────────────────────────────────────────────────────

/// Per-IP rate limiter shared across all API routes.
/// 100 requests/minute per IP (burst of 20).
pub type IpRateLimiter = Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

/// Server-side state store for Discord profile-import OAuth.
/// Maps opaque CSRF token → (user_id, created_at).
/// Keeping this separate from oauth_states prevents user_id leakage in the URL.
pub type DiscordImportStateStore = dashmap::DashMap<String, (uuid::Uuid, std::time::Instant)>;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub hub: Arc<Hub>,
    pub rate_limiter: IpRateLimiter,
    pub trusted_proxy_ips: Arc<HashSet<IpAddr>>,
    pub jwt_keys: Arc<JwtKeys>,
    pub login_limiter: Arc<LoginRateLimiter>,
    /// Short-lived CSRF state tokens for OAuth flows
    pub oauth_states: Arc<OAuthStateStore>,
    /// State tokens for the Discord profile-import flow: csrf_token → (user_id, created_at)
    pub discord_import_states: Arc<DiscordImportStateStore>,
    /// Shared HTTP client — reuses TLS sessions and connection pools across requests.
    pub http_client: reqwest::Client,
}

// ─── Router builders ───────────────────────────────────────────────────────────

/// Build the full application router, ready to be served or used by integration tests.
///
/// Integration tests call this instead of `main()` to get a `Router` without
/// binding a TCP socket:
///
/// ```ignore
/// let server = axum_test::TestServer::new(build_router(state)).unwrap();
/// ```
pub fn build_router(state: AppState) -> Router {
    let api_v1 = api_router().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        api_rate_limit_check,
    ));
    let api_v2 = api_router_v2().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        api_rate_limit_check,
    ));

    Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(hub::ws_handler))
        .nest("/auth/oauth", auth::oauth_router())
        .nest("/api/v1", api_v1)
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

pub(crate) fn api_router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/account", users::account_router())
        .nest("/servers", servers::router())
        .nest("/channels", channels::router())
        .nest("/conversations", messages::router())
        .nest("/keys", keys::router())
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
        .layer(axum::middleware::from_fn(csrf::csrf_check))
}

pub(crate) fn api_router_v2() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::v2_router())
        .nest("/devices", devices::router())
        .nest("/keys", keys::v2_router())
        .nest("/conversations", messages::v2_router())
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
            // x-refresh-token is only used by native clients (Tauri/Capacitor) where
            // cross-origin cookies are unreliable. Web browser clients use HttpOnly cookies
            // and never send this header. The backend only returns refresh tokens in JSON
            // for native device platforms.
            HeaderName::from_static("x-refresh-token"),
        ])
        .allow_credentials(true)
}

pub(crate) async fn api_rate_limit_check(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    let peer_ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip());
    let client_ip = auth::handlers::extract_ip(req.headers(), peer_ip, &state);
    if state.rate_limiter.check_key(&client_ip).is_err() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(req).await)
}

/// Load trusted reverse-proxy IPs from the `TRUSTED_PROXY_IPS` env var.
///
/// Always includes `127.0.0.1` and `::1`. Additional IPs are parsed from
/// a comma-separated list. Invalid entries are logged and skipped.
///
/// # Returns
///
/// Set of IPs whose `X-Forwarded-For` headers are trusted for client-IP extraction.
pub fn load_trusted_proxy_ips() -> HashSet<IpAddr> {
    let mut ips = HashSet::from([
        IpAddr::from([127, 0, 0, 1]),
        IpAddr::from(std::net::Ipv6Addr::LOCALHOST),
    ]);
    if let Ok(raw) = std::env::var("TRUSTED_PROXY_IPS") {
        for entry in raw
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            if let Ok(ip) = entry.parse::<IpAddr>() {
                ips.insert(ip);
            } else {
                tracing::warn!("Ignoring invalid TRUSTED_PROXY_IPS entry: {entry}");
            }
        }
    }
    ips
}

/// Read a `NonZeroU32` from the environment, falling back to `default`.
///
/// # Panics
///
/// Panics if `default` is zero (compile-time logic error).
pub fn env_non_zero_u32(name: &str, default: u32) -> NonZeroU32 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .and_then(NonZeroU32::new)
        .unwrap_or_else(|| NonZeroU32::new(default).expect("default quota must be non-zero"))
}

pub(crate) async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = state.db.ping().await.is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({ "status": if db_ok { "ok" } else { "degraded" }, "db": db_ok })),
    )
}
