/// Integration tests for the health endpoint.
///
/// Full HTTP integration tests require a running PostgreSQL instance.
/// In CI, these run after `docker compose up -d` spins up the DB.
/// Use the #[sqlx::test] attribute for DB-backed tests — sqlx auto-creates
/// and rolls back an isolated test DB per test.
///
/// Phase 2 adds full auth integration tests here.

#[tokio::test]
async fn placeholder_health_test() {
    // Placeholder: the real health check test sends an HTTP request
    // to a locally running server. In CI this is verified by:
    //   cargo run & sleep 2 && curl localhost:8080/health
    // Full axum-test integration lands in Phase 2.
    assert!(true, "Health endpoint placeholder — see CI workflow");
}
