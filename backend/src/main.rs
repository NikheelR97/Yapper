use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod error;
mod hub;
mod auth;
mod users;
mod servers;
mod channels;
mod messages;
mod keys;
mod media;
mod canvas;
mod emojis;
mod parental;
mod screentime;
mod bots;
mod discord;
mod notifications;

use db::Database;
use hub::Hub;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub hub: Arc<Hub>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env in development
    dotenvy::dotenv().ok();

    // Structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "yapper_server=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let db = Database::connect(&database_url).await?;
    db.run_migrations().await?;

    let hub = Arc::new(Hub::new());
    let state = AppState { db, hub };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(hub::ws_handler))
        .nest("/api/v1", api_router())
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors_layer())
        .with_state(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{host}:{port}");

    tracing::info!("Yapper server listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn api_router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/servers", servers::router())
        .nest("/channels", channels::router())
        .nest("/keys", keys::router())
        .nest("/media", media::router())
        .nest("/canvas", canvas::router())
        .nest("/emojis", emojis::router())
        .nest("/parental", parental::router())
        .nest("/screentime", screentime::router())
        .nest("/bots", bots::router())
        .nest("/discord", discord::router())
        .nest("/notifications", notifications::router())
}

fn cors_layer() -> CorsLayer {
    use axum::http::{HeaderValue, Method};
    use tower_http::cors::Any;

    let origins: Vec<HeaderValue> = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| {
            "http://localhost:5173,tauri://localhost,capacitor://localhost".to_string()
        })
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers(Any)
        .allow_credentials(true)
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = state.db.ping().await.is_ok();
    let status = if db_ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(json!({ "status": if db_ok { "ok" } else { "degraded" }, "db": db_ok })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_app_state_is_clone() {
        // Compile-time check that AppState implements Clone (needed for Axum)
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }
}
