pub mod handlers;
pub mod service;

use axum::{routing::get, Router};

use crate::AppState;

/// Mounted at /api/v1/channels — handles message operations.
/// Channel CRUD (list/create) lives in servers::router() under /:id/channels.
pub fn router() -> Router<AppState> {
    Router::new().route("/:id/messages", get(handlers::get_messages).post(handlers::send_message))
}
