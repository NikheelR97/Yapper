use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};

use crate::{auth::AuthUser, error::AppResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/me", get(get_me))
}

/// GET /api/v1/users/me — returns the authenticated user's profile.
async fn get_me(auth: AuthUser, State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let user = sqlx::query!(
        r#"
        SELECT id, username, display_name, avatar_url, account_type, is_premium
        FROM users WHERE id = $1 AND deleted_at IS NULL
        "#,
        auth.user_id,
    )
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(crate::error::AppError::NotFound(
        "User not found".to_string(),
    ))?;

    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "avatar_url": user.avatar_url,
        "account_type": user.account_type,
        "is_premium": user.is_premium,
    })))
}
