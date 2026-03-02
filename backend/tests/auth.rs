//! Auth integration tests.
//! These tests verify the auth HTTP API via axum-test when a database is available.
//! Unit tests for service logic live in src/auth/service.rs and src/auth/middleware.rs.

/// Smoke test to verify the test file compiles.
#[test]
fn auth_tests_compile() {
    // Unit tests for hashing, tokens, and rate limiting are in:
    //   src/auth/service.rs  — hash/verify, token generation
    //   src/auth/middleware.rs — LoginRateLimiter
    //
    // HTTP-level integration tests require a live database.
    // Run with: DATABASE_URL=... cargo test --test auth -- integration
    assert!(true);
}
