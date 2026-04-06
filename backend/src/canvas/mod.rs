pub mod handlers;
pub mod service;
pub mod types;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

use crate::AppState;
use handlers::*;

pub fn router() -> Router<AppState> {
    Router::new()
        // ─── Music ───────────────────────────────────────────────────────
        .route("/canvas/servers/:server_id/music", patch(patch_music))
        .route(
            "/canvas/servers/:server_id/music/queue",
            post(enqueue_track),
        )
        .route(
            "/canvas/servers/:server_id/music/queue/:track_id",
            delete(remove_from_queue),
        )
        .route(
            "/canvas/servers/:server_id/music/queue/reorder",
            post(reorder_queue),
        )
        .route(
            "/canvas/servers/:server_id/music/skip-vote",
            post(skip_vote),
        )
        .route(
            "/canvas/servers/:server_id/music/advance",
            post(advance_track),
        )
        .route(
            "/canvas/servers/:server_id/music/history",
            get(get_music_history),
        )
        .route(
            "/canvas/servers/:server_id/music/settings",
            patch(update_music_settings),
        )
        .route(
            "/canvas/servers/:server_id/music/dj/:user_id",
            put(grant_dj).delete(revoke_dj),
        )
        // ─── Polls ───────────────────────────────────────────────────────
        .route("/canvas/channels/:channel_id/polls", post(create_poll))
        .route("/canvas/polls/:poll_id/vote", post(vote_poll))
        .route("/canvas/polls/:poll_id/close", post(close_poll))
        .route("/canvas/polls/:poll_id/results", get(get_poll_results))
        // ─── Clips ───────────────────────────────────────────────────────
        .route(
            "/canvas/clips/:clip_id/reactions",
            put(add_clip_reaction).delete(remove_clip_reaction),
        )
        .route("/canvas/servers/:server_id/pinned-clips", post(pin_clip))
        .route(
            "/canvas/servers/:server_id/pinned-clips/:clip_id",
            delete(unpin_clip),
        )
        // ─── Events ─────────────────────────────────────────────────────
        .route("/canvas/servers/:server_id/events", post(create_event))
        .route(
            "/canvas/events/:event_id",
            patch(update_event).delete(delete_event),
        )
        // ─── Hydration ──────────────────────────────────────────────────
        .route("/canvas/servers/:server_id/state", get(get_canvas_state))
        // ─── Legacy ─────────────────────────────────────────────────────
        .route("/canvas/servers/:server_id", get(get_canvas_legacy))
}
