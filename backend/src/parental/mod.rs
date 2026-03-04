/**
 * Parental Controls — COPPA-compliant child account management.
 *
 * Routes (mounted under /api/v1/parental):
 *   POST  /children                             — create child account (COPPA consent)
 *   GET   /children                             — list managed children
 *   GET   /children/:child_id/overview          — child activity snapshot
 *   GET   /children/:child_id/notifications     — pending alerts (friend reqs + server joins)
 *   PATCH /friend-requests/:id/approve          — approve pending friend request
 *   PATCH /friend-requests/:id/decline          — decline pending friend request
 *   PATCH /server-joins/:id/approve             — approve pending server join
 *   PATCH /server-joins/:id/decline             — decline pending server join
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

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
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

// ─── Create child account ────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct CreateChildInput {
    username: String,
    display_name: String,
    email: String,
    password: String,
    date_of_birth: String, // ISO date YYYY-MM-DD
}

/// POST /api/v1/parental/children
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

    // Validate DOB — must be under 18
    let dob: chrono::NaiveDate = body
        .date_of_birth
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid date_of_birth — use YYYY-MM-DD".into()))?;
    let today = chrono::Utc::now().date_naive();
    let age_years = today.years_since(dob).unwrap_or(99);
    if age_years >= 18 {
        return Err(AppError::BadRequest(
            "Child accounts require a date of birth under 18 years ago".into(),
        ));
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
    .bind(&body.username)
    .bind(&body.display_name)
    .bind(&body.email)
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

    // Link parent → child
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
            "username":     body.username,
            "display_name": body.display_name,
            "account_type": "child",
            "parental_controls_enabled": true,
        })),
    ))
}

// ─── List children ────────────────────────────────────────────────────────────

/// GET /api/v1/parental/children
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

// ─── Child overview ───────────────────────────────────────────────────────────

/// GET /api/v1/parental/children/:child_id/overview
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

// ─── Notifications ────────────────────────────────────────────────────────────

/// GET /api/v1/parental/children/:child_id/notifications
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

// ─── Approval / Decline ───────────────────────────────────────────────────────

/// PATCH /api/v1/parental/friend-requests/:id/approve
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

/// PATCH /api/v1/parental/friend-requests/:id/decline
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

/// PATCH /api/v1/parental/server-joins/:id/approve
async fn approve_server_join(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(join_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let req = fetch_server_join(join_id, auth.user_id, &state).await?;
    let child_id: Uuid = req.try_get("child_user_id")?;
    let server_id: Uuid = req.try_get("server_id")?;

    sqlx::query(
        "UPDATE pending_server_joins SET status = 'approved', reviewed_at = NOW() WHERE id = $1",
    )
    .bind(join_id)
    .execute(state.db.pool())
    .await?;

    // Direct membership insert — bypasses parental check (parent already approved)
    sqlx::query(
        "INSERT INTO server_memberships (user_id, server_id, role) VALUES ($1, $2, 'member') \
         ON CONFLICT (user_id, server_id) DO NOTHING",
    )
    .bind(child_id)
    .bind(server_id)
    .execute(state.db.pool())
    .await?;

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

/// PATCH /api/v1/parental/server-joins/:id/decline
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

// ─── Private helpers ──────────────────────────────────────────────────────────

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
