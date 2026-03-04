pub mod handlers;
pub mod service;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(handlers::create_server).get(handlers::list_my_servers),
        )
        .route(
            "/:id",
            get(handlers::get_server).patch(handlers::update_server),
        )
        .route("/:id/join", post(handlers::join_server))
        .route("/:id/leave", delete(handlers::leave_server))
        .route("/:id/invite", post(handlers::create_invite))
        .route("/join/:code", post(handlers::join_by_invite))
        // Channel CRUD nested under server (/:id/channels)
        .route(
            "/:id/channels",
            get(crate::channels::handlers::list_channels)
                .post(crate::channels::handlers::create_channel),
        )
        // Custom emoji CRUD nested under server (/:id/emojis)
        .nest("/:id/emojis", crate::emojis::server_emoji_router())
}
