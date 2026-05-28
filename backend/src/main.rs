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

const SENTRY_REDACTED: &str = "[redacted]";

/// Scrub PII from Sentry events before they leave the process.
///
/// Strips sensitive request metadata and redacts JWTs, cookies, CSRF/refresh
/// tokens, Signal key material, Argon2 password hashes, emails, and message
/// payload fields before the event leaves the process.
///
/// # Security invariants
///
/// * No JWT, session cookie, refresh token, or email address is ever
///   transmitted to Sentry. This is required for GDPR Art. 5(1)(c)
///   (data minimisation) and prevents credential leakage to a third party.
fn scrub_sentry_event(
    mut event: sentry::protocol::Event<'static>,
) -> Option<sentry::protocol::Event<'static>> {
    // Scrub sensitive headers from request data
    if let Some(ref mut request) = event.request {
        request.headers.retain(|k, _| !is_sensitive_sentry_key(k));
        request.env.retain(|k, _| !is_sensitive_sentry_key(k));
        request.cookies = None;
        scrub_sentry_string_option(&mut request.data);
        scrub_sentry_string_option(&mut request.query_string);
    }

    // Scrub user email if captured
    if let Some(ref mut user) = event.user {
        user.email = None;
        user.other.retain(|k, v| {
            if is_sensitive_sentry_key(k) {
                return false;
            }
            scrub_sentry_value(v);
            true
        });
    }

    // Scrub sensitive keys from extra data
    event.extra.retain(|k, v| {
        if is_sensitive_sentry_key(k) {
            return false;
        }
        scrub_sentry_value(v);
        true
    });
    scrub_sentry_string_option(&mut event.message);
    if let Some(ref mut logentry) = event.logentry {
        scrub_sentry_string(&mut logentry.message);
        logentry.params.iter_mut().for_each(scrub_sentry_value);
    }

    // Scrub breadcrumb data that may contain PII
    for breadcrumb in &mut event.breadcrumbs {
        breadcrumb.data.retain(|k, v| {
            if is_sensitive_sentry_key(k) {
                return false;
            }
            scrub_sentry_value(v);
            true
        });
        scrub_sentry_string_option(&mut breadcrumb.message);
    }

    // Scrub exception values that may embed PII
    for exc in &mut event.exception.values {
        scrub_sentry_string_option(&mut exc.value);
    }

    // Scrub tags
    event.tags.retain(|k, v| {
        if is_sensitive_sentry_key(k) {
            return false;
        }
        scrub_sentry_string(v);
        true
    });

    Some(event)
}

fn is_sensitive_sentry_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "authorization",
        "cookie",
        "set-cookie",
        "csrf",
        "token",
        "password",
        "password_hash",
        "argon2",
        "secret",
        "private_key",
        "jwt",
        "email",
        "message",
        "description",
        "body",
        "plaintext",
        "ciphertext",
        "identity_dh_key",
        "identity_sig_key",
        "signed_prekey",
        "one_time_prekey",
        "key",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn scrub_sentry_value(value: &mut sentry::protocol::Value) {
    match value {
        sentry::protocol::Value::String(value) => scrub_sentry_string(value),
        sentry::protocol::Value::Array(values) => values.iter_mut().for_each(scrub_sentry_value),
        sentry::protocol::Value::Object(values) => {
            values.retain(|k, v| {
                if is_sensitive_sentry_key(k) {
                    return false;
                }
                scrub_sentry_value(v);
                true
            });
        }
        _ => {}
    }
}

fn scrub_sentry_string_option(value: &mut Option<String>) {
    if let Some(value) = value {
        scrub_sentry_string(value);
    }
}

fn scrub_sentry_string(value: &mut String) {
    if contains_sensitive_sentry_text(value) {
        *value = SENTRY_REDACTED.to_string();
    }
}

fn contains_sensitive_sentry_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    (value.contains('@') && value.contains('.'))
        || lower.contains("bearer ")
        || lower.contains("$argon2")
        || lower.contains("refresh_token")
        || lower.contains("csrf_token")
        || lower.contains("set-cookie")
        || lower.contains("authorization")
        || lower.contains("password_hash")
        || lower.contains("identity_dh_key")
        || lower.contains("identity_sig_key")
        || lower.contains("signed_prekey")
        || lower.contains("one_time_prekey")
        || lower.contains("private_key")
        || lower.contains("jwt")
        || looks_like_jwt(value)
}

fn looks_like_jwt(value: &str) -> bool {
    value
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | ';' | '(' | ')'))
        .any(|part| part.len() >= 24 && part.starts_with("eyJ") && part.split('.').count() == 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentry::protocol::{Breadcrumb, Event, Exception, Request, User, Value};

    #[test]
    fn scrub_sentry_event_removes_sensitive_request_and_user_data() {
        let mut event = Event::default();
        let mut request = Request {
            data: Some("email=alice@example.com&csrf_token=abc".to_string()),
            query_string: Some("jwt=eyJhbGciOiJSUzI1NiJ9.payload.signature".to_string()),
            cookies: Some("refresh_token=secret".to_string()),
            ..Default::default()
        };
        request
            .headers
            .insert("authorization".to_string(), "Bearer token".to_string());
        request
            .headers
            .insert("x-request-id".to_string(), "safe-id".to_string());
        event.request = Some(request);
        event.user = Some(User {
            email: Some("alice@example.com".to_string()),
            ..Default::default()
        });

        let scrubbed = scrub_sentry_event(event).expect("event should be kept");
        let json = serde_json::to_string(&scrubbed).expect("serialize event");

        assert!(!json.contains("alice@example.com"));
        assert!(!json.contains("Bearer token"));
        assert!(!json.contains("refresh_token"));
        assert!(!json.contains("eyJhbGciOiJSUzI1NiJ9"));
        assert!(json.contains("safe-id"));
        assert!(json.contains(SENTRY_REDACTED));
    }

    #[test]
    fn scrub_sentry_event_redacts_sensitive_values_under_generic_keys() {
        let mut event = Event::default();
        event.extra.insert(
            "detail".to_string(),
            Value::String("Bearer eyJhbGciOiJSUzI1NiJ9.payload.signature".to_string()),
        );
        event.extra.insert(
            "safe".to_string(),
            Value::String("public diagnostic".to_string()),
        );
        event.breadcrumbs.values.push(Breadcrumb {
            message: Some("identity_dh_key=abc signed_prekey=def".to_string()),
            ..Default::default()
        });
        event.exception.values.push(Exception {
            ty: "Error".to_string(),
            value: Some("password_hash=$argon2id$v=19$m=19456,t=2,p=1$abc$def".to_string()),
            ..Default::default()
        });

        let scrubbed = scrub_sentry_event(event).expect("event should be kept");
        let json = serde_json::to_string(&scrubbed).expect("serialize event");

        assert!(!json.contains("Bearer"));
        assert!(!json.contains("eyJhbGciOiJSUzI1NiJ9"));
        assert!(!json.contains("identity_dh_key"));
        assert!(!json.contains("signed_prekey"));
        assert!(!json.contains("$argon2id"));
        assert!(json.contains("public diagnostic"));
        assert!(json.contains(SENTRY_REDACTED));
    }
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
            // SENTRY_ENVIRONMENT takes precedence so non-Fly deploys (e.g. the
            // Coolify staging stack) can report their own environment. Falls
            // back to the original Fly.io auto-detection when unset.
            environment: Some(
                std::env::var("SENTRY_ENVIRONMENT")
                    .unwrap_or_else(|_| {
                        if std::env::var("FLY_APP_NAME").is_ok() {
                            "production".to_string()
                        } else {
                            "development".to_string()
                        }
                    })
                    .into(),
            ),
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
