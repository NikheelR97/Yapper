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
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

// Security header constants
const NOSNIFF: &str = "nosniff";
const DENY_FRAME: &str = "DENY";
const HSTS: &str = "max-age=63072000; includeSubDomains; preload";
const CSP_API: &str = "default-src 'none'; frame-ancestors 'none'";
use sentry::integrations::tracing as sentry_tracing;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Per-IP rate limiter shared across all API routes.
/// 100 requests/minute per IP (burst of 20).
pub type IpRateLimiter = Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

mod auth;
mod bots;
mod canvas;
mod channels;
mod csrf;
mod db;
mod devices;
mod discord;
mod emojis;
mod error;
mod explore;
mod hub;
mod keys;
mod media;
mod messages;
mod notifications;
mod parental;
mod premium;
mod screentime;
mod servers;
mod support;
mod users;

use auth::{JwtKeys, LoginRateLimiter, OAuthStateStore};
use db::Database;
use hub::Hub;

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env in development
    dotenvy::dotenv().ok();

    // Sentry error monitoring — no-op if SENTRY_DSN is absent
    let _sentry_guard = sentry::init((
        std::env::var("SENTRY_DSN").unwrap_or_default(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(if std::env::var("FLY_APP_NAME").is_ok() {
                "production".into()
            } else {
                "development".into()
            }),
            ..Default::default()
        },
    ));

    // Structured logging (+ Sentry breadcrumb integration)
    tracing_subscriber::registry()
        .with(sentry_tracing::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "yapper_server=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = Database::connect(&database_url).await?;
    db.run_migrations().await?;

    // Initialise Cloudflare R2 client (reads R2_* env vars).
    // Panics at startup if vars are missing — intentional fail-fast.
    if std::env::var("R2_ACCOUNT_ID").is_ok() {
        media::init_r2().await;
    } else {
        tracing::warn!("R2_ACCOUNT_ID not set — media upload URLs will not work");
    }

    let hub = Arc::new(Hub::new());

    // Per-IP rate limiter: 100 requests/min, burst of 20
    let quota = governor::Quota::per_minute(NonZeroU32::new(100).unwrap())
        .allow_burst(NonZeroU32::new(20).unwrap());
    let rate_limiter: IpRateLimiter = Arc::new(RateLimiter::keyed(quota));
    let trusted_proxy_ips = Arc::new(load_trusted_proxy_ips());
    let jwt_keys = Arc::new(JwtKeys::from_env()?);
    let login_limiter = Arc::new(LoginRateLimiter::new());
    let oauth_states = Arc::new(OAuthStateStore::new());
    let discord_import_states = Arc::new(DiscordImportStateStore::new());

    let state = AppState {
        db,
        hub,
        rate_limiter,
        trusted_proxy_ips,
        jwt_keys,
        login_limiter,
        oauth_states,
        discord_import_states,
    };

    let api_v1 = api_router().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        api_rate_limit_check,
    ));
    let api_v2 = api_router_v2().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        api_rate_limit_check,
    ));

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(hub::ws_handler))
        // OAuth at top level — must match redirect URIs registered in Discord/Google consoles
        .nest("/auth/oauth", auth::oauth_router())
        .nest("/api/v1", api_v1)
        .nest("/api/v2", api_v2)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        // Security response headers
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
        .with_state(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{host}:{port}");

    tracing::info!("Yapper server listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn load_trusted_proxy_ips() -> HashSet<IpAddr> {
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

async fn api_rate_limit_check(
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

fn api_router() -> Router<AppState> {
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

fn api_router_v2() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::v2_router())
        .nest("/devices", devices::router())
        .nest("/keys", keys::v2_router())
        .nest("/conversations", messages::v2_router())
        .layer(axum::middleware::from_fn(csrf::csrf_check))
}

fn cors_layer() -> CorsLayer {
    use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
    use axum::http::HeaderName;

    let origins: Vec<HeaderValue> = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| {
            "http://localhost:5173,tauri://localhost,capacitor://localhost".to_string()
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
        ])
        .allow_credentials(true)
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_is_clone() {
        // Compile-time check that AppState implements Clone (needed for Axum)
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }
}
