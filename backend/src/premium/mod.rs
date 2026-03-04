mod handlers;
mod service;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::get_status).delete(handlers::cancel))
        .route("/activate", post(handlers::activate))
        .route("/webhook", post(handlers::stripe_webhook))
}
