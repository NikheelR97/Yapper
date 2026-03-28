/**
 * Parental Controls â€” COPPA-compliant child account management.
 *
 * Routes (mounted under /api/v2/parental):
 *   POST  /children                             â€” create child account (COPPA consent)
 *   GET   /children                             â€” list managed children
 *   GET   /children/:child_id/overview          â€” child activity snapshot
 *   GET   /children/:child_id/notifications     â€” pending alerts (friend reqs + server joins)
 *   PATCH /friend-requests/:id/approve          â€” approve pending friend request
 *   PATCH /friend-requests/:id/decline          â€” decline pending friend request
 *   PATCH /server-joins/:id/approve             â€” approve pending server join
 *   PATCH /server-joins/:id/decline             â€” decline pending server join
 */
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use sqlx::Row;
use uuid::Uuid;

const MAX_USERNAME_LEN: usize = 32;
const MIN_USERNAME_LEN: usize = 2;
const MAX_DISPLAY_NAME_LEN: usize = 50;
const MAX_EMAIL_LEN: usize = 254;
const MAX_PASSWORD_BYTES: usize = 1024;
const MIN_PASSWORD_LEN: usize = 8;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    servers::service::{consume_invite_use, insert_server_member_with_cap},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/children", post(create_child).get(list_children))
        .route("/children/:child_id/overview", get(get_child_overview))
        .route("/children/:child_id/notifications", get(get_notifications))
        .route(
            "/friend-requests/:id/approve",
            patch(approve_friend_request),
        )
        .route(
            "/friend-requests/:id/decline",
            patch(decline_friend_request),
        )
        .route("/server-joins/:id/approve", patch(approve_server_join))
        .route("/server-joins/:id/decline", patch(decline_server_join))
}

// â”€â”€â”€ Create child account â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateChildInput {
    username: String,
    display_name: String,
    email: String,
    password: String,
    date_of_birth: String, // ISO date YYYY-MM-DD
}

/// POST /api/v2/parental/children
async fn create_child(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateChildInput>,
) -> AppResult<impl IntoResponse> {
    // Caller must be standard or parent
    let caller_row =
        sqlx::query("SELECT account_type FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(auth.user_id)
            .fetch_optional(state.db.pool())
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let account_type: String = caller_row.try_get("account_type")?;
    if account_type == "child" || account_type == "bot" {
        return Err(AppError::Forbidden);
    }

    // Validate DOB â€” must be under 18
    let dob: chrono::NaiveDate = body
        .date_of_birth
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid date_of_birth â€” use YYYY-MM-DD".into()))?;
    let today = chrono::Utc::now().date_naive();
    let age_years = today.years_since(dob).unwrap_or(99);
    if age_years >= 18 {
        return Err(AppError::BadRequest(
            "Child accounts require a date of birth under 18 years ago".into(),
        ));
    }

    // Validate string fields before any DB or crypto work (Rule 3 â€” validate at boundaries)
    if body.password.len() < MIN_PASSWORD_LEN || body.password.len() > MAX_PASSWORD_BYTES {
        return Err(AppError::BadRequest(format!(
            "Password must be {MIN_PASSWORD_LEN}â€“{MAX_PASSWORD_BYTES} characters"
        )));
    }
    let username = body.username.trim();
    if username.len() < MIN_USERNAME_LEN || username.len() > MAX_USERNAME_LEN {
        return Err(AppError::BadRequest(format!(
            "Username must be {MIN_USERNAME_LEN}â€“{MAX_USERNAME_LEN} characters"
        )));
    }
    let display_name = body.display_name.trim();
    if display_name.is_empty() || display_name.len() > MAX_DISPLAY_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "Display name must be 1â€“{MAX_DISPLAY_NAME_LEN} characters"
        )));
    }
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || email.len() > MAX_EMAIL_LEN || !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }

    // Hash password using existing auth helper
    let password_hash = crate::auth::service::hash_password(&body.password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    // Insert child user
    let child_row = sqlx::query(
        "INSERT INTO users
            (username, display_name, email, password_hash, account_type,
             parental_controls_enabled, date_of_birth, coppa_consent_verified_at)
         VALUES ($1, $2, $3, $4, 'child', TRUE, $5, NOW())
         RETURNING id",
    )
    .bind(username)
    .bind(display_name)
    .bind(email)
    .bind(&password_hash)
    .bind(dob)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref de) = e {
            if de.code().as_deref() == Some("23505") {
                return AppError::Conflict("Username or email already taken".into());
            }
        }
        AppError::Database(e)
    })?;

    let child_id: Uuid = child_row.try_get("id")?;

    // Upgrade caller to parent if needed
    if account_type == "standard" {
        sqlx::query("UPDATE users SET account_type = 'parent' WHERE id = $1")
            .bind(auth.user_id)
            .execute(state.db.pool())
            .await?;
    }

    // Link parent â†’ child
    sqlx::query(
        "INSERT INTO parent_child_relationships (parent_user_id, child_user_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(child_id)
    .execute(state.db.pool())
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "child_id":     child_id,
            "username":     username,
            "display_name": display_name,
            "account_type": "child",
            "parental_controls_enabled": true,
        })),
    ))
}

// â”€â”€â”€ List children â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /api/v2/parental/children
async fn list_children(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT u.id, u.username, u.display_name, u.avatar_url, u.date_of_birth, u.last_seen_at
         FROM parent_child_relationships pcr
         JOIN users u ON u.id = pcr.child_user_id
         WHERE pcr.parent_user_id = $1 AND u.deleted_at IS NULL
         ORDER BY u.display_name",
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let children: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({
            "id":           r.try_get::<Uuid, _>("id").ok(),
            "username":     r.try_get::<String, _>("username").unwrap_or_default(),
            "display_name": r.try_get::<String, _>("display_name").unwrap_or_default(),
            "avatar_url":   r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
            "date_of_birth": r.try_get::<Option<chrono::NaiveDate>, _>("date_of_birth")
                               .ok().flatten().map(|d| d.to_string()),
            "last_seen_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_seen_at")
                              .ok().flatten().map(|t| t.to_rfc3339()),
        }))
        .collect();

    Ok(Json(serde_json::json!({ "children": children })))
}

// â”€â”€â”€ Child overview â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /api/v2/parental/children/:child_id/overview
async fn get_child_overview(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(child_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    require_parent_of(auth.user_id, child_id, &state).await?;

    let child_row = sqlx::query(
        "SELECT id, username, display_name, avatar_url, date_of_birth, last_seen_at,
                parental_controls_enabled
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(child_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Child account not found".into()))?;

    let pending_friends: i64 = sqlx::query(
        "SELECT COUNT(*) FROM pending_friend_requests WHERE child_user_id = $1 AND status = 'pending'",
    )
    .bind(child_id)
    .fetch_one(state.db.pool())
    .await
    .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
    .unwrap_or(0);

    let pending_joins: i64 = sqlx::query(
        "SELECT COUNT(*) FROM pending_server_joins WHERE child_user_id = $1 AND status = 'pending'",
    )
    .bind(child_id)
    .fetch_one(state.db.pool())
    .await
    .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
    .unwrap_or(0);

    let server_rows = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.icon_url
         FROM server_memberships sm
         JOIN servers s ON s.id = sm.server_id
         WHERE sm.user_id = $1
         ORDER BY sm.joined_at DESC
         LIMIT 5",
    )
    .bind(child_id)
    .fetch_all(state.db.pool())
    .await?;

    let servers: Vec<serde_json::Value> = server_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":       r.try_get::<Uuid, _>("id").ok(),
                "name":     r.try_get::<String, _>("name").unwrap_or_default(),
                "slug":     r.try_get::<String, _>("slug").unwrap_or_default(),
                "icon_url": r.try_get::<Option<String>, _>("icon_url").ok().flatten(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "id":           child_row.try_get::<Uuid, _>("id").ok(),
        "username":     child_row.try_get::<String, _>("username").unwrap_or_default(),
        "display_name": child_row.try_get::<String, _>("display_name").unwrap_or_default(),
        "avatar_url":   child_row.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
        "date_of_birth": child_row.try_get::<Option<chrono::NaiveDate>, _>("date_of_birth")
                           .ok().flatten().map(|d| d.to_string()),
        "last_seen_at": child_row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_seen_at")
                          .ok().flatten().map(|t| t.to_rfc3339()),
        "parental_controls_enabled": child_row.try_get::<bool, _>("parental_controls_enabled").unwrap_or(true),
        "pending_friend_requests": pending_friends,
        "pending_server_joins":    pending_joins,
        "top_servers": servers,
    })))
}

// â”€â”€â”€ Notifications â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /api/v2/parental/children/:child_id/notifications
async fn get_notifications(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(child_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    require_parent_of(auth.user_id, child_id, &state).await?;

    let friend_rows = sqlx::query(
        "SELECT id, requester_id, requester_name, status, created_at
         FROM pending_friend_requests
         WHERE child_user_id = $1
         ORDER BY created_at DESC LIMIT 50",
    )
    .bind(child_id)
    .fetch_all(state.db.pool())
    .await?;

    let friend_requests: Vec<serde_json::Value> = friend_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":             r.try_get::<Uuid, _>("id").ok(),
                "requester_id":   r.try_get::<Uuid, _>("requester_id").ok(),
                "requester_name": r.try_get::<String, _>("requester_name").unwrap_or_default(),
                "status":         r.try_get::<String, _>("status").unwrap_or_default(),
                "created_at":     r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                    .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    let join_rows = sqlx::query(
        "SELECT id, server_id, server_name, invite_code, status, created_at
         FROM pending_server_joins
         WHERE child_user_id = $1
         ORDER BY created_at DESC LIMIT 50",
    )
    .bind(child_id)
    .fetch_all(state.db.pool())
    .await?;

    let server_joins: Vec<serde_json::Value> = join_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":          r.try_get::<Uuid, _>("id").ok(),
                "server_id":   r.try_get::<Uuid, _>("server_id").ok(),
                "server_name": r.try_get::<String, _>("server_name").unwrap_or_default(),
                "invite_code": r.try_get::<Option<String>, _>("invite_code").ok().flatten(),
                "status":      r.try_get::<String, _>("status").unwrap_or_default(),
                "created_at":  r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                 .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    // Mark as read
    if let Err(e) = sqlx::query(
        "UPDATE parent_notifications SET read = TRUE
         WHERE parent_user_id = $1 AND child_user_id = $2 AND read = FALSE",
    )
    .bind(auth.user_id)
    .bind(child_id)
    .execute(state.db.pool())
    .await
    {
        tracing::warn!("Failed to mark notifications read: {e}");
    }

    Ok(Json(serde_json::json!({
        "friend_requests": friend_requests,
        "server_joins":    server_joins,
    })))
}

// â”€â”€â”€ Approval / Decline â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// PATCH /api/v2/parental/friend-requests/:id/approve
async fn approve_friend_request(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(req_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let req = fetch_friend_request(req_id, auth.user_id, &state).await?;
    let child_id: Uuid = req.try_get("child_user_id")?;
    let requester_id: Uuid = req.try_get("requester_id")?;

    sqlx::query(
        "UPDATE pending_friend_requests SET status = 'approved', reviewed_at = NOW() WHERE id = $1",
    )
    .bind(req_id)
    .execute(state.db.pool())
    .await?;

    // Create friendship
    let (uid1, uid2) = if child_id < requester_id {
        (child_id, requester_id)
    } else {
        (requester_id, child_id)
    };
    sqlx::query(
        "INSERT INTO friendships (user_id_1, user_id_2, status) VALUES ($1, $2, 'accepted')
         ON CONFLICT (user_id_1, user_id_2) DO UPDATE SET status = 'accepted'",
    )
    .bind(uid1)
    .bind(uid2)
    .execute(state.db.pool())
    .await?;

    audit(auth.user_id, child_id, "approve_friend", req_id, &state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/v2/parental/friend-requests/:id/decline
async fn decline_friend_request(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(req_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let req = fetch_friend_request(req_id, auth.user_id, &state).await?;
    let child_id: Uuid = req.try_get("child_user_id")?;

    sqlx::query(
        "UPDATE pending_friend_requests SET status = 'declined', reviewed_at = NOW() WHERE id = $1",
    )
    .bind(req_id)
    .execute(state.db.pool())
    .await?;

    audit(auth.user_id, child_id, "decline_friend", req_id, &state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/v2/parental/server-joins/:id/approve
async fn approve_server_join(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(join_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let req = fetch_server_join(join_id, auth.user_id, &state).await?;
    let child_id: Uuid = req.try_get("child_user_id")?;
    let server_id: Uuid = req.try_get("server_id")?;
    let invite_code: Option<String> = req.try_get("invite_code")?;

    let mut tx = state.db.pool().begin().await?;

    let reviewed = sqlx::query(
        "UPDATE pending_server_joins SET status = 'approved', reviewed_at = NOW() WHERE id = $1 AND status = 'pending' RETURNING id",
    )
    .bind(join_id)
    .fetch_optional(&mut *tx)
    .await?;
    if reviewed.is_none() {
        tx.rollback().await.ok();
        return Err(AppError::BadRequest("Request already reviewed".into()));
    }

    insert_server_member_with_cap(&mut tx, child_id, server_id).await?;
    consume_invite_use(&mut tx, invite_code.as_deref()).await?;
    tx.commit().await?;

    audit(
        auth.user_id,
        child_id,
        "approve_server_join",
        join_id,
        &state,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/v2/parental/server-joins/:id/decline
async fn decline_server_join(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(join_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let req = fetch_server_join(join_id, auth.user_id, &state).await?;
    let child_id: Uuid = req.try_get("child_user_id")?;

    sqlx::query(
        "UPDATE pending_server_joins SET status = 'declined', reviewed_at = NOW() WHERE id = $1",
    )
    .bind(join_id)
    .execute(state.db.pool())
    .await?;

    audit(
        auth.user_id,
        child_id,
        "decline_server_join",
        join_id,
        &state,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// â”€â”€â”€ Private helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

async fn require_parent_of(parent_id: Uuid, child_id: Uuid, state: &AppState) -> AppResult<()> {
    let ok = sqlx::query(
        "SELECT 1 FROM parent_child_relationships
         WHERE parent_user_id = $1 AND child_user_id = $2",
    )
    .bind(parent_id)
    .bind(child_id)
    .fetch_optional(state.db.pool())
    .await?
    .is_some();

    if !ok {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn fetch_friend_request(
    req_id: Uuid,
    parent_id: Uuid,
    state: &AppState,
) -> AppResult<sqlx::postgres::PgRow> {
    let row = sqlx::query(
        "SELECT pfr.id, pfr.child_user_id, pfr.requester_id, pfr.status
         FROM pending_friend_requests pfr
         JOIN parent_child_relationships pcr ON pcr.child_user_id = pfr.child_user_id
         WHERE pfr.id = $1 AND pcr.parent_user_id = $2",
    )
    .bind(req_id)
    .bind(parent_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Request not found or not yours to review".into()))?;

    if row.try_get::<String, _>("status").unwrap_or_default() != "pending" {
        return Err(AppError::BadRequest("Request already reviewed".into()));
    }
    Ok(row)
}

async fn fetch_server_join(
    join_id: Uuid,
    parent_id: Uuid,
    state: &AppState,
) -> AppResult<sqlx::postgres::PgRow> {
    let row = sqlx::query(
        "SELECT psj.id, psj.child_user_id, psj.server_id, psj.invite_code, psj.status
         FROM pending_server_joins psj
         JOIN parent_child_relationships pcr ON pcr.child_user_id = psj.child_user_id
         WHERE psj.id = $1 AND pcr.parent_user_id = $2",
    )
    .bind(join_id)
    .bind(parent_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Request not found or not yours to review".into()))?;

    if row.try_get::<String, _>("status").unwrap_or_default() != "pending" {
        return Err(AppError::BadRequest("Request already reviewed".into()));
    }
    Ok(row)
}

async fn audit(
    parent_id: Uuid,
    child_id: Uuid,
    action: &str,
    reference_id: Uuid,
    state: &AppState,
) {
    let _ = sqlx::query(
        "INSERT INTO parental_action_audit (parent_user_id, child_user_id, action, reference_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(parent_id)
    .bind(child_id)
    .bind(action)
    .bind(reference_id)
    .execute(state.db.pool())
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    // â”€â”€â”€ Validation constants sanity checks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn constants_are_sensible() {
        assert!(MIN_USERNAME_LEN > 0);
        assert!(MIN_USERNAME_LEN < MAX_USERNAME_LEN);
        assert!(MIN_PASSWORD_LEN > 0);
        assert!(MIN_PASSWORD_LEN < MAX_PASSWORD_BYTES);
        assert!(MAX_DISPLAY_NAME_LEN > 0);
        assert!(MAX_EMAIL_LEN > 0);
    }

    // â”€â”€â”€ CreateChildInput deserialization â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn valid_json() -> &'static str {
        r#"{
            "username": "kiddo",
            "display_name": "Cool Kid",
            "email": "kid@example.com",
            "password": "securepass123",
            "date_of_birth": "2015-06-15"
        }"#
    }

    #[test]
    fn deserialize_valid_input() {
        let input: CreateChildInput = serde_json::from_str(valid_json()).unwrap();
        assert_eq!(input.username, "kiddo");
        assert_eq!(input.display_name, "Cool Kid");
        assert_eq!(input.email, "kid@example.com");
        assert_eq!(input.password, "securepass123");
        assert_eq!(input.date_of_birth, "2015-06-15");
    }

    #[test]
    fn deserialize_rejects_unknown_fields() {
        let json = r#"{
            "username": "kiddo",
            "display_name": "Cool Kid",
            "email": "kid@example.com",
            "password": "securepass123",
            "date_of_birth": "2015-06-15",
            "extra_field": "should fail"
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("unknown field"),
            "Expected 'unknown field' error, got: {err_msg}"
        );
    }

    #[test]
    fn deserialize_rejects_missing_username() {
        let json = r#"{
            "display_name": "Cool Kid",
            "email": "kid@example.com",
            "password": "securepass123",
            "date_of_birth": "2015-06-15"
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("username"),
            "Expected missing 'username' error, got: {err_msg}"
        );
    }

    #[test]
    fn deserialize_rejects_missing_display_name() {
        let json = r#"{
            "username": "kiddo",
            "email": "kid@example.com",
            "password": "securepass123",
            "date_of_birth": "2015-06-15"
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("display_name"),
            "Expected missing 'display_name' error, got: {err_msg}"
        );
    }

    #[test]
    fn deserialize_rejects_missing_email() {
        let json = r#"{
            "username": "kiddo",
            "display_name": "Cool Kid",
            "password": "securepass123",
            "date_of_birth": "2015-06-15"
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("email"),
            "Expected missing 'email' error, got: {err_msg}"
        );
    }

    #[test]
    fn deserialize_rejects_missing_password() {
        let json = r#"{
            "username": "kiddo",
            "display_name": "Cool Kid",
            "email": "kid@example.com",
            "date_of_birth": "2015-06-15"
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("password"),
            "Expected missing 'password' error, got: {err_msg}"
        );
    }

    #[test]
    fn deserialize_rejects_missing_date_of_birth() {
        let json = r#"{
            "username": "kiddo",
            "display_name": "Cool Kid",
            "email": "kid@example.com",
            "password": "securepass123"
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("date_of_birth"),
            "Expected missing 'date_of_birth' error, got: {err_msg}"
        );
    }

    #[test]
    fn deserialize_rejects_empty_json_object() {
        let result = serde_json::from_str::<CreateChildInput>("{}");
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_null() {
        let result = serde_json::from_str::<CreateChildInput>("null");
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_array() {
        let result = serde_json::from_str::<CreateChildInput>("[]");
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_wrong_type_for_username() {
        let json = r#"{
            "username": 123,
            "display_name": "Cool Kid",
            "email": "kid@example.com",
            "password": "securepass123",
            "date_of_birth": "2015-06-15"
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_wrong_type_for_date_of_birth() {
        // date_of_birth is a String, not a boolean
        let json = r#"{
            "username": "kiddo",
            "display_name": "Cool Kid",
            "email": "kid@example.com",
            "password": "securepass123",
            "date_of_birth": false
        }"#;
        let result = serde_json::from_str::<CreateChildInput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_accepts_empty_string_values() {
        // serde will accept empty strings â€” validation happens later in the handler
        let json = r#"{
            "username": "",
            "display_name": "",
            "email": "",
            "password": "",
            "date_of_birth": ""
        }"#;
        let input: CreateChildInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.username, "");
        assert_eq!(input.password, "");
    }

    #[test]
    fn deserialize_preserves_whitespace_in_fields() {
        // Handler calls .trim() â€” serde should preserve raw whitespace
        let json = r#"{
            "username": "  spaced  ",
            "display_name": "  Kid  ",
            "email": "  kid@example.com  ",
            "password": "pass with spaces",
            "date_of_birth": "2015-06-15"
        }"#;
        let input: CreateChildInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.username, "  spaced  ");
        assert_eq!(input.display_name, "  Kid  ");
        assert_eq!(input.email, "  kid@example.com  ");
    }

    #[test]
    fn deserialize_accepts_unicode_values() {
        let json = r#"{
            "username": "ã“ã©ã‚‚",
            "display_name": "å­ä¾›ã¡ã‚ƒã‚“",
            "email": "kid@ä¾‹ãˆ.jp",
            "password": "ãƒ‘ã‚¹ãƒ¯ãƒ¼ãƒ‰12345",
            "date_of_birth": "2015-06-15"
        }"#;
        let input: CreateChildInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.username, "ã“ã©ã‚‚");
        assert_eq!(input.display_name, "å­ä¾›ã¡ã‚ƒã‚“");
    }

    // â”€â”€â”€ COPPA age validation logic (inline in handler) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Mirrors the DOB parsing + age check from create_child.
    /// Tests the same logic without needing DB/state.
    fn validate_child_age(date_of_birth: &str) -> Result<u32, &'static str> {
        let dob: chrono::NaiveDate = date_of_birth
            .parse()
            .map_err(|_| "Invalid date_of_birth â€” use YYYY-MM-DD")?;
        let today = chrono::Utc::now().date_naive();
        let age_years = today.years_since(dob).unwrap_or(99);
        if age_years >= 18 {
            return Err("Child accounts require a date of birth under 18 years ago");
        }
        Ok(age_years)
    }

    #[test]
    fn coppa_rejects_adult_dob() {
        // 30 years ago â€” definitely 18+
        let dob = (chrono::Utc::now().date_naive() - chrono::Months::new(12 * 30))
            .format("%Y-%m-%d")
            .to_string();
        assert!(validate_child_age(&dob).is_err());
    }

    #[test]
    fn coppa_rejects_exactly_18() {
        // Exactly 18 years ago today
        let dob = (chrono::Utc::now().date_naive() - chrono::Months::new(12 * 18))
            .format("%Y-%m-%d")
            .to_string();
        assert!(validate_child_age(&dob).is_err());
    }

    #[test]
    fn coppa_accepts_under_18() {
        // 10 years ago
        let dob = (chrono::Utc::now().date_naive() - chrono::Months::new(12 * 10))
            .format("%Y-%m-%d")
            .to_string();
        let result = validate_child_age(&dob);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn coppa_accepts_just_under_18() {
        // 17 years ago
        let dob = (chrono::Utc::now().date_naive() - chrono::Months::new(12 * 17))
            .format("%Y-%m-%d")
            .to_string();
        let result = validate_child_age(&dob);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 17);
    }

    #[test]
    fn coppa_accepts_newborn() {
        // Today's date as DOB
        let dob = chrono::Utc::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let result = validate_child_age(&dob);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn coppa_rejects_invalid_date_format() {
        assert!(validate_child_age("15-06-2015").is_err()); // DD-MM-YYYY
        assert!(validate_child_age("06/15/2015").is_err()); // MM/DD/YYYY
        assert!(validate_child_age("not-a-date").is_err());
        assert!(validate_child_age("").is_err());
        assert!(validate_child_age("2015-13-01").is_err()); // month 13
        assert!(validate_child_age("2015-02-29").is_err()); // 2015 not a leap year
    }

    #[test]
    fn coppa_rejects_future_date() {
        // Future DOB â€” years_since returns None â†’ unwrap_or(99) â†’ >= 18 â†’ rejected
        let future = (chrono::Utc::now().date_naive() + chrono::Months::new(12))
            .format("%Y-%m-%d")
            .to_string();
        assert!(validate_child_age(&future).is_err());
    }

    // â”€â”€â”€ Inline validation logic (mirrors handler checks) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn validate_password(password: &str) -> Result<(), String> {
        if password.len() < MIN_PASSWORD_LEN || password.len() > MAX_PASSWORD_BYTES {
            return Err(format!(
                "Password must be {MIN_PASSWORD_LEN}â€“{MAX_PASSWORD_BYTES} characters"
            ));
        }
        Ok(())
    }

    #[test]
    fn password_too_short() {
        assert!(validate_password("").is_err());
        assert!(validate_password("1234567").is_err()); // 7 chars
    }

    #[test]
    fn password_exactly_min() {
        assert!(validate_password("12345678").is_ok()); // 8 chars = MIN_PASSWORD_LEN
    }

    #[test]
    fn password_valid_length() {
        assert!(validate_password("a_reasonable_password").is_ok());
    }

    #[test]
    fn password_too_long() {
        let long = "x".repeat(MAX_PASSWORD_BYTES + 1);
        assert!(validate_password(&long).is_err());
    }

    #[test]
    fn password_exactly_max() {
        let max = "x".repeat(MAX_PASSWORD_BYTES);
        assert!(validate_password(&max).is_ok());
    }

    fn validate_username(raw: &str) -> Result<(), String> {
        let username = raw.trim();
        if username.len() < MIN_USERNAME_LEN || username.len() > MAX_USERNAME_LEN {
            return Err(format!(
                "Username must be {MIN_USERNAME_LEN}â€“{MAX_USERNAME_LEN} characters"
            ));
        }
        Ok(())
    }

    #[test]
    fn username_too_short() {
        assert!(validate_username("").is_err());
        assert!(validate_username("a").is_err()); // 1 char < MIN_USERNAME_LEN (2)
    }

    #[test]
    fn username_trims_to_too_short() {
        assert!(validate_username("   a   ").is_err()); // trims to 1 char
        assert!(validate_username("     ").is_err()); // trims to 0 chars
    }

    #[test]
    fn username_exactly_min() {
        assert!(validate_username("ab").is_ok()); // 2 chars = MIN_USERNAME_LEN
    }

    #[test]
    fn username_valid() {
        assert!(validate_username("cool_kid").is_ok());
    }

    #[test]
    fn username_too_long() {
        let long = "x".repeat(MAX_USERNAME_LEN + 1);
        assert!(validate_username(&long).is_err());
    }

    #[test]
    fn username_exactly_max() {
        let max = "x".repeat(MAX_USERNAME_LEN);
        assert!(validate_username(&max).is_ok());
    }

    fn validate_display_name(raw: &str) -> Result<(), String> {
        let display_name = raw.trim();
        if display_name.is_empty() || display_name.len() > MAX_DISPLAY_NAME_LEN {
            return Err(format!(
                "Display name must be 1â€“{MAX_DISPLAY_NAME_LEN} characters"
            ));
        }
        Ok(())
    }

    #[test]
    fn display_name_empty() {
        assert!(validate_display_name("").is_err());
    }

    #[test]
    fn display_name_whitespace_only() {
        assert!(validate_display_name("   ").is_err()); // trims to empty
    }

    #[test]
    fn display_name_valid() {
        assert!(validate_display_name("Cool Kid").is_ok());
    }

    #[test]
    fn display_name_too_long() {
        let long = "x".repeat(MAX_DISPLAY_NAME_LEN + 1);
        assert!(validate_display_name(&long).is_err());
    }

    #[test]
    fn display_name_exactly_max() {
        let max = "x".repeat(MAX_DISPLAY_NAME_LEN);
        assert!(validate_display_name(&max).is_ok());
    }

    fn validate_email(raw: &str) -> Result<(), String> {
        let email = raw.trim();
        if email.is_empty() || email.len() > MAX_EMAIL_LEN || !email.contains('@') {
            return Err("Invalid email address".into());
        }
        Ok(())
    }

    #[test]
    fn email_empty() {
        assert!(validate_email("").is_err());
    }

    #[test]
    fn email_missing_at() {
        assert!(validate_email("invalid-email.com").is_err());
    }

    #[test]
    fn email_valid() {
        assert!(validate_email("kid@example.com").is_ok());
    }

    #[test]
    fn email_too_long() {
        let long = format!("{}@example.com", "x".repeat(MAX_EMAIL_LEN));
        assert!(validate_email(&long).is_err());
    }

    #[test]
    fn email_whitespace_only() {
        assert!(validate_email("   ").is_err()); // trims to empty
    }

    #[test]
    fn email_at_only() {
        // "@" alone has an '@' but is technically accepted by this simple check
        assert!(validate_email("@").is_ok());
    }

    // â”€â”€â”€ Friendship UID ordering (from approve_friend_request) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn ordered_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }

    #[test]
    fn uid_pair_ordering_is_deterministic() {
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let (u1, u2) = ordered_pair(a, b);
        assert_eq!(u1, a);
        assert_eq!(u2, b);

        // Reversed input should produce the same order
        let (u1r, u2r) = ordered_pair(b, a);
        assert_eq!(u1r, a);
        assert_eq!(u2r, b);
    }

    #[test]
    fn uid_pair_ordering_same_id() {
        let a = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let (u1, u2) = ordered_pair(a, a);
        assert_eq!(u1, a);
        assert_eq!(u2, a);
    }
}
