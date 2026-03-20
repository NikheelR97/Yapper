//! Yapper server binary entry point.
//!
//! All application logic lives in `lib.rs`. This file is a thin wrapper that
//! reads environment variables, constructs `AppState`, and starts the TCP listener.

use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use yapper_server::{
    auth::{JwtKeys, LoginRateLimiter, OAuthStateStore},
    build_router, db::Database, env_non_zero_u32, hub::Hub, load_trusted_proxy_ips,
    media, AppState, DiscordImportStateStore, IpRateLimiter,
};

use sentry::integrations::tracing as sentry_tracing;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    // Per-IP rate limiter defaults stay production-safe, but local E2E can
    // raise them via env vars so browser bootstraps do not self-throttle.
    let quota = governor::Quota::per_minute(env_non_zero_u32("API_RATE_LIMIT_PER_MINUTE", 100))
        .allow_burst(env_non_zero_u32("API_RATE_LIMIT_BURST", 20));
    let rate_limiter: IpRateLimiter = Arc::new(governor::RateLimiter::keyed(quota));
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

    let app = build_router(state);

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
