use axum::{extract::{Path, State}, response::IntoResponse, routing::get, Json, Router};
use sqlx::Row;
use uuid::Uuid;

use crate::{auth::AuthUser, error::AppResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me))
        .route("/:id/presence", get(get_presence))
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

/// GET /api/v1/users/:id/presence — online status + last seen timestamp.
async fn get_presence(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    debug_assert!(user_id != Uuid::nil());

    let online = state.hub.is_online(&user_id);

    let row = sqlx::query(
        "SELECT last_seen_at FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?;

    let last_seen_at: Option<String> = row.as_ref().and_then(|r| {
        r.try_get::<chrono::DateTime<chrono::Utc>, _>("last_seen_at")
            .ok()
            .map(|dt| dt.to_rfc3339())
    });

    Ok(Json(serde_json::json!({
        "online": online,
        "last_seen_at": last_seen_at,
    })))
}
