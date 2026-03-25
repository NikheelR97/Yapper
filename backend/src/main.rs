#![deny(warnings)]
//! Yapper server binary entry point.
//!
//! All application logic lives in `lib.rs`. This file is a thin wrapper that
//! reads environment variables, constructs `AppState`, and starts the TCP listener.

use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use yapper_server::{
    auth::{JwtKeys, LoginRateLimiter, OAuthStateStore},
    build_router,
    db::Database,
    env_non_zero_u32,
    hub::Hub,
    load_trusted_proxy_ips, media, AppState, DiscordImportStateStore, IpRateLimiter,
};

use sentry::integrations::tracing as sentry_tracing;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Scrub PII from Sentry events before they leave the process.
/// Strips sensitive headers (Authorization, Cookie) and redacts fields
/// that may contain tokens, passwords, or email addresses.
fn scrub_sentry_event(
    mut event: sentry::protocol::Event<'static>,
) -> Option<sentry::protocol::Event<'static>> {
    // Scrub sensitive headers from request data
    if let Some(ref mut request) = event.request {
        let sensitive_headers = ["authorization", "cookie", "set-cookie", "x-refresh-token"];
        request.headers.retain(|k, _| {
            !sensitive_headers.contains(&k.to_lowercase().as_str())
        });
    }

    // Scrub user email if captured
    if let Some(ref mut user) = event.user {
        user.email = None;
    }

    // Scrub sensitive keys from extra data
    let sensitive_keys = ["password", "token", "secret", "key", "refresh_token", "email"];
    event.extra.retain(|k, _| {
        !sensitive_keys.iter().any(|s| k.to_lowercase().contains(s))
    });

    // Scrub breadcrumb data that may contain PII
    for breadcrumb in &mut event.breadcrumbs {
        breadcrumb.data.retain(|k, _| {
            !sensitive_keys.iter().any(|s| k.to_lowercase().contains(s))
        });
        // Redact message content that looks like an email
        if let Some(ref msg) = breadcrumb.message {
            if msg.contains('@') && msg.contains('.') {
                breadcrumb.message = Some("[redacted]".to_string());
            }
        }
    }

    // Scrub exception values that may embed PII
    for exc in &mut event.exception.values {
        if let Some(ref val) = exc.value {
            if val.contains('@') && val.contains('.') {
                exc.value = Some("[redacted — may contain PII]".to_string());
            }
        }
    }

    // Scrub tags
    event.tags.retain(|k, _| {
        !sensitive_keys.iter().any(|s| k.to_lowercase().contains(s))
    });

    Some(event)
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
            before_send: Some(std::sync::Arc::new(scrub_sentry_event)),
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
    db.start_keepalive();

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

    // GC task: shrink all keyed rate limiters and hub caches every 5 minutes
    // to prevent unbounded memory growth on the 256MB VM.
    {
        let rl = Arc::clone(&rate_limiter);
        let hub_gc = Arc::clone(&hub);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            interval.tick().await; // skip immediate first tick
            loop {
                interval.tick().await;
                rl.retain_recent();
                yapper_server::auth::handlers::gc_auth_rate_limiters();
                yapper_server::hub::gc_ws_rate_limiter();
                hub_gc.gc_caches();
                tracing::debug!("GC: retained recent rate limiter + hub cache entries");
            }
        });
    }

    let trusted_proxy_ips = Arc::new(load_trusted_proxy_ips());
    let jwt_keys = Arc::new(JwtKeys::from_env()?);
    let login_limiter = Arc::new(LoginRateLimiter::new());
    let oauth_states = Arc::new(OAuthStateStore::new());
    let discord_import_states = Arc::new(DiscordImportStateStore::new());

    let http_client = reqwest::Client::new();

    let state = AppState {
        db,
        hub,
        rate_limiter,
        trusted_proxy_ips,
        jwt_keys,
        login_limiter,
        oauth_states,
        discord_import_states,
        http_client,
    };

    {
        let retention_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(error) = yapper_server::retention::run_cleanup(&retention_state).await {
                    tracing::warn!("Retention cleanup failed: {error}");
                }
            }
        });
    }

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
