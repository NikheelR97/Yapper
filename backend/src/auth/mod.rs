use axum::Router;
use uuid::Uuid;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
}

/// JWT claims returned after token validation.
#[derive(Debug)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
}

/// Validates an RS256 JWT access token.
/// Returns Claims on success, or an error if invalid/expired.
/// Full implementation in Phase 2 (auth module).
pub async fn validate_access_token(
    _token: &str,
    _pool: &sqlx::PgPool,
) -> anyhow::Result<Claims> {
    // TODO: Phase 2 — parse & verify RS256 JWT, check exp, return Claims
    anyhow::bail!("Auth not yet implemented — Phase 2")
}
