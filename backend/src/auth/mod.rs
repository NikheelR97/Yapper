pub mod handlers;
pub mod middleware;
pub mod service;

use axum::{routing::{delete, get, post}, Router};

use crate::AppState;

pub use middleware::{AuthUser, LoginRateLimiter, OptionalAuthUser};
pub use service::{AccessClaims, JwtKeys};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh))
        .route("/logout", delete(handlers::logout))
        .route("/verify-email", get(handlers::verify_email))
        .route("/password-reset/request", post(handlers::request_password_reset))
        // OAuth routes wired in S2 Week 6
        // .route("/oauth/discord", get(oauth::discord_redirect))
        // .route("/oauth/discord/callback", get(oauth::discord_callback))
}

/// Called by the WebSocket hub to validate the first-message auth token.
pub fn validate_ws_token(token: &str, keys: &JwtKeys) -> Option<uuid::Uuid> {
    service::validate_access_token(token, keys)
        .ok()
        .map(|t| t.claims.sub)
}
