pub mod handlers;
pub mod middleware;
pub mod oauth;
pub mod service;
pub mod v2;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub use middleware::{AuthDevice, AuthUser, LoginRateLimiter};
pub use oauth::OAuthStateStore;
pub use service::JwtKeys;

/// Device-aware auth routes mounted under `/api/v2/auth/*`.
pub fn v2_router() -> Router<AppState> {
    v2::router()
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

/// OAuth routes are versionless to match configured provider redirect URIs.
pub fn oauth_router() -> Router<AppState> {
    Router::new()
        .route("/discord", get(oauth::discord_redirect))
        .route("/discord/callback", get(oauth::discord_callback))
        .route("/google", get(oauth::google_redirect))
        .route("/google/callback", get(oauth::google_callback))
        .route("/apple", get(oauth::apple_redirect))
        .route("/apple/callback", post(oauth::apple_callback))
}
