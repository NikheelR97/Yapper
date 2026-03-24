pub mod handlers;
pub mod service;

use axum::{extract::DefaultBodyLimit, routing::get, Router};

use crate::AppState;

/// Mounted at /api/v1/channels — handles message and key-distribution operations.
/// Channel CRUD (list/create) lives in servers::router() under /:id/channels.
pub fn router() -> Router<AppState> {
    let message_routes = Router::new()
        .route(
            "/:id/messages",
            get(handlers::get_messages).post(handlers::send_message),
        )
        .layer(DefaultBodyLimit::max(
            crate::constants::MAX_MESSAGE_REQUEST_BODY_SIZE,
        ));

    Router::new()
        .merge(message_routes)
        .route("/:id/members", get(handlers::list_members))
        .route(
            "/:id/sender-key-dist",
            get(handlers::get_key_dists).post(handlers::post_key_dists),
        )
}
