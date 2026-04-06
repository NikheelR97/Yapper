//! Integration tests for the `/health` liveness probe.
//!
//! Audit ref: MED-001 — verifies that `/health` returns 200 only when the
//! database pool answers `SELECT 1` within the 2 s timeout, and 503 otherwise.
//! Prior to this fix the handler returned 200 unconditionally.

use super::{build_test_state_from_pool, spawn_test_server_from_pool, spawn_test_server_from_state};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn health_returns_ok_when_database_is_alive(pool: PgPool) {
    let Some(server) = spawn_test_server_from_pool(pool).await else {
        return;
    };

    let response = server.get("/health").await;

    assert_eq!(
        response.status_code().as_u16(),
        200,
        "expected 200 with live pool, got {}: {}",
        response.status_code(),
        response.text()
    );

    let body: serde_json::Value = response.json();
    assert_eq!(body["ok"], serde_json::json!(true));
    assert_eq!(body["db"], serde_json::json!("ok"));
}

#[sqlx::test(migrations = "./migrations")]
async fn health_returns_503_when_database_is_unreachable(pool: PgPool) {
    let Some(state) = build_test_state_from_pool(pool.clone()).await else {
        return;
    };

    // Close the pool BEFORE building the test server. PgPool is Arc-internally,
    // so the close propagates to the cloned reference held by AppState.
    pool.close().await;
    assert!(pool.is_closed(), "pool should be closed");

    let server = spawn_test_server_from_state(state);
    let response = server.get("/health").await;

    assert_eq!(
        response.status_code().as_u16(),
        503,
        "expected 503 with closed pool, got {}: {}",
        response.status_code(),
        response.text()
    );

    let body: serde_json::Value = response.json();
    assert_eq!(body["ok"], serde_json::json!(false));
    assert_eq!(body["db"], serde_json::json!("down"));
}
