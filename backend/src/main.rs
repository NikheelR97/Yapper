#![deny(warnings)]

use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, RateLimiter};
use serde_json::json;
use std::{net::IpAddr, num::NonZeroU32, sync::Arc};
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
mod screentime;
mod servers;
mod users;

use auth::{JwtKeys, LoginRateLimiter, OAuthStateStore};
use db::Database;
use hub::Hub;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub hub: Arc<Hub>,
    pub rate_limiter: IpRateLimiter,
    pub jwt_keys: Arc<JwtKeys>,
    pub login_limiter: Arc<LoginRateLimiter>,
    /// Short-lived CSRF state tokens for OAuth flows
    pub oauth_states: Arc<OAuthStateStore>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env in development
    dotenvy::dotenv().ok();

    // Structured logging
    tracing_subscriber::registry()
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
    let jwt_keys = Arc::new(JwtKeys::from_env()?);
    let login_limiter = Arc::new(LoginRateLimiter::new());
    let oauth_states = Arc::new(OAuthStateStore::new());

    let state = AppState {
        db,
        hub,
        rate_limiter,
        jwt_keys,
        login_limiter,
        oauth_states,
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(hub::ws_handler))
        // OAuth at top level — must match redirect URIs registered in Discord/Google consoles
        .nest("/auth/oauth", auth::oauth_router())
        .nest("/api/v1", api_router())
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
    axum::serve(listener, app).await?;

    Ok(())
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
        .nest("/notifications", notifications::router())
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
