pub mod handlers;
pub mod middleware;
pub mod oauth;
pub mod service;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::AppState;

pub use middleware::{AuthUser, LoginRateLimiter};
pub use oauth::OAuthStateStore;
pub use service::JwtKeys;

/// Core auth routes — nested under /api/v1/auth/
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh))
        .route("/logout", delete(handlers::logout))
        .route("/verify-email", get(handlers::verify_email))
        .route(
            "/password-reset/request",
            post(handlers::request_password_reset),
        )
        .route(
            "/password-reset/confirm",
            post(handlers::confirm_password_reset),
        )
        .route("/change-password", post(handlers::change_password))
}

/// OAuth routes — registered at top level (/auth/oauth/...) to match
/// the redirect URIs configured in Discord and Google developer consoles.
pub fn oauth_router() -> Router<AppState> {
    Router::new()
        .route("/discord", get(oauth::discord_redirect))
        .route("/discord/callback", get(oauth::discord_callback))
        .route("/google", get(oauth::google_redirect))
        .route("/google/callback", get(oauth::google_callback))
}

/// Called by the WebSocket hub to validate the first-message auth token.
pub fn validate_ws_token(token: &str, keys: &JwtKeys) -> Option<uuid::Uuid> {
    service::validate_access_token(token, keys)
        .ok()
        .map(|t| t.claims.sub)
}
