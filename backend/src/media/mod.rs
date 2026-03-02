use axum::{routing::post, Router};

use crate::AppState;

mod handlers;
pub mod r2;

/// Initialise the Cloudflare R2 client singleton.
/// Must be called once during `main()` before serving requests.
pub use r2::init_r2;

pub fn router() -> Router<AppState> {
    Router::new().route("/upload-url", post(handlers::upload_url))
}
