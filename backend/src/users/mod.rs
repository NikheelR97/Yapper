/**
 * Users — profile, presence, follow graph, hype moments, settings.
 *
 * Routes (mounted under /api/v1/users):
 *   GET    /me                              — own full profile
 *   PATCH  /me                              — update profile fields
 *   PATCH  /me/username                     — change username (30-day cooldown)
 *   GET    /me/privacy                      — fetch privacy settings
 *   PATCH  /me/privacy                      — update privacy settings
 *   GET    /:id/presence                    — online status + last seen
 *   GET    /by/:username                    — public profile (counts, mutuals, communities)
 *   POST   /by/:username/follow             — follow a user
 *   DELETE /by/:username/follow             — unfollow
 *   POST   /by/:username/friend-request     — send friend request (parental-intercepted)
 *   GET    /me/feed                         — activity feed from followed users
 *   POST   /me/hype-moments                 — pin a message to own profile
 *   GET    /by/:username/hype-moments       — pinned messages for a profile
 *
 * Routes (mounted under /api/v1/account):
 *   GET    /data-export                     — GDPR data export (JSON)
 *   DELETE /                                — soft-delete account
 */

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::Utc;
use regex::Regex;
use once_cell::sync::Lazy;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

// ─── Validation constants ─────────────────────────────────────────────────────

const MAX_DISPLAY_NAME: usize = 50;
const MAX_ABOUT_ME: usize = 500;
const MAX_LOCATION: usize = 100;
const USERNAME_COOLDOWN_DAYS: i64 = 30;

static USERNAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]{3,32}$").unwrap());

static HEX_COLOR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap());

static HTTPS_URL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^https://").unwrap());

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me).patch(update_profile))
        .route("/me/username", patch(change_username))
        .route("/me/privacy", get(get_privacy).patch(update_privacy))
        .route("/me/feed", get(get_feed))
        .route("/me/hype-moments", post(pin_hype_moment))
        .route("/:id/presence", get(get_presence))
        .route("/by/:username", get(get_profile))
        .route("/by/:username/follow", post(follow_user))
        .route("/by/:username/follow", delete(unfollow_user))
        .route("/by/:username/friend-request", post(send_friend_request))
        .route("/by/:username/hype-moments", get(get_hype_moments))
}

/// Sub-router mounted at /api/v1/account in main.rs
pub fn account_router() -> Router<AppState> {
    Router::new()
        .route("/data-export", get(gdpr_export))
        .route("/", delete(delete_account))
}

// ─── Own profile ──────────────────────────────────────────────────────────────

/// GET /api/v1/users/me
async fn get_me(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query(
        "SELECT id, username, display_name, avatar_url, banner_url, about_me, location,
                profile_theme_color, account_type, is_premium, parental_controls_enabled,
                created_at
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    Ok(Json(serde_json::json!({
        "id":                        row.try_get::<Uuid, _>("id").ok(),
        "username":                  row.try_get::<String, _>("username").unwrap_or_default(),
        "display_name":              row.try_get::<String, _>("display_name").unwrap_or_default(),
        "avatar_url":                row.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
        "banner_url":                row.try_get::<Option<String>, _>("banner_url").ok().flatten(),
        "about_me":                  row.try_get::<Option<String>, _>("about_me").ok().flatten(),
        "location":                  row.try_get::<Option<String>, _>("location").ok().flatten(),
        "profile_theme_color":       row.try_get::<Option<String>, _>("profile_theme_color").ok().flatten(),
        "account_type":              row.try_get::<String, _>("account_type").unwrap_or_default(),
        "is_premium":                row.try_get::<bool, _>("is_premium").unwrap_or(false),
        "parental_controls_enabled": row.try_get::<bool, _>("parental_controls_enabled").unwrap_or(false),
        "created_at":                row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                        .ok().map(|t| t.to_rfc3339()),
    })))
}

// ─── Presence ─────────────────────────────────────────────────────────────────

/// GET /api/v1/users/:id/presence
async fn get_presence(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let online = state.hub.is_online(&user_id);
    let away = online && state.hub.is_away(&user_id);

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
        "away":   away,
        "last_seen_at": last_seen_at,
    })))
}

// ─── Public profile ───────────────────────────────────────────────────────────

/// GET /api/v1/users/by/:username
async fn get_profile(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query(
        "SELECT id, username, display_name, avatar_url, banner_url, about_me, location,
                profile_theme_color, account_type, is_premium, created_at
         FROM users WHERE username = $1 AND deleted_at IS NULL",
    )
    .bind(&username)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("User @{username} not found")))?;

    let profile_id: Uuid = row.try_get("id")?;

    // Follower / following counts
    let counts = sqlx::query(
        "SELECT
            (SELECT COUNT(*) FROM followers WHERE following_id = $1) AS follower_count,
            (SELECT COUNT(*) FROM followers WHERE follower_id   = $1) AS following_count",
    )
    .bind(profile_id)
    .fetch_one(state.db.pool())
    .await?;

    let follower_count: i64 = counts.try_get("follower_count").unwrap_or(0);
    let following_count: i64 = counts.try_get("following_count").unwrap_or(0);

    // Is the calling user following this profile?
    let is_following: bool = sqlx::query(
        "SELECT 1 FROM followers WHERE follower_id = $1 AND following_id = $2",
    )
    .bind(auth.user_id)
    .bind(profile_id)
    .fetch_optional(state.db.pool())
    .await?
    .is_some();

    // Mutual followers (people who follow both the viewer and the profile)
    let mutual_rows = sqlx::query(
        "SELECT u.id, u.username, u.display_name, u.avatar_url
         FROM followers f1
         JOIN followers f2 ON f1.follower_id = f2.follower_id
         JOIN users u ON u.id = f1.follower_id
         WHERE f1.following_id = $1
           AND f2.following_id = $2
           AND u.deleted_at IS NULL
           AND u.id != $2
         LIMIT 6",
    )
    .bind(profile_id)
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let mutuals: Vec<serde_json::Value> = mutual_rows
        .iter()
        .map(|r| serde_json::json!({
            "id":           r.try_get::<Uuid, _>("id").ok(),
            "username":     r.try_get::<String, _>("username").unwrap_or_default(),
            "display_name": r.try_get::<Option<String>, _>("display_name").ok().flatten(),
            "avatar_url":   r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
        }))
        .collect();

    // Top public servers this user is in
    let server_rows = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.icon_url, s.tags,
                COUNT(sm2.user_id) AS member_count
         FROM server_memberships sm
         JOIN servers s ON s.id = sm.server_id
         LEFT JOIN server_memberships sm2 ON sm2.server_id = s.id
         WHERE sm.user_id = $1 AND s.is_public = TRUE
         GROUP BY s.id
         ORDER BY member_count DESC
         LIMIT 5",
    )
    .bind(profile_id)
    .fetch_all(state.db.pool())
    .await?;

    let top_communities: Vec<serde_json::Value> = server_rows
        .iter()
        .map(|r| serde_json::json!({
            "id":           r.try_get::<Uuid, _>("id").ok(),
            "name":         r.try_get::<String, _>("name").unwrap_or_default(),
            "slug":         r.try_get::<String, _>("slug").unwrap_or_default(),
            "icon_url":     r.try_get::<Option<String>, _>("icon_url").ok().flatten(),
            "tags":         r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
            "member_count": r.try_get::<i64, _>("member_count").unwrap_or(0),
        }))
        .collect();

    Ok(Json(serde_json::json!({
        "id":                  profile_id,
        "username":            row.try_get::<String, _>("username").unwrap_or_default(),
        "display_name":        row.try_get::<String, _>("display_name").unwrap_or_default(),
        "avatar_url":          row.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
        "banner_url":          row.try_get::<Option<String>, _>("banner_url").ok().flatten(),
        "about_me":            row.try_get::<Option<String>, _>("about_me").ok().flatten(),
        "location":            row.try_get::<Option<String>, _>("location").ok().flatten(),
        "profile_theme_color": row.try_get::<Option<String>, _>("profile_theme_color").ok().flatten(),
        "account_type":        row.try_get::<String, _>("account_type").unwrap_or_default(),
        "is_premium":          row.try_get::<bool, _>("is_premium").unwrap_or(false),
        "follower_count":      follower_count,
        "following_count":     following_count,
        "is_following":        is_following,
        "mutual_followers":    mutuals,
        "top_communities":     top_communities,
        "created_at":          row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                  .ok().map(|t| t.to_rfc3339()),
    })))
}

// ─── Follow / Unfollow ────────────────────────────────────────────────────────

/// POST /api/v1/users/by/:username/follow
async fn follow_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> AppResult<impl IntoResponse> {
    let target = resolve_user(&username, &state).await?;
    if target == auth.user_id {
        return Err(AppError::BadRequest("Cannot follow yourself".into()));
    }
    sqlx::query(
        "INSERT INTO followers (follower_id, following_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(target)
    .execute(state.db.pool())
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/users/by/:username/follow
async fn unfollow_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> AppResult<impl IntoResponse> {
    let target = resolve_user(&username, &state).await?;
    sqlx::query(
        "DELETE FROM followers WHERE follower_id = $1 AND following_id = $2",
    )
    .bind(auth.user_id)
    .bind(target)
    .execute(state.db.pool())
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── Friend requests ─────────────────────────────────────────────────────────

/// POST /api/v1/users/by/:username/friend-request
/// Parental interception: if target has parental_controls_enabled → pending approval.
pub async fn send_friend_request(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> AppResult<impl IntoResponse> {
    let target_id = resolve_user(&username, &state).await?;
    if target_id == auth.user_id {
        return Err(AppError::BadRequest("Cannot send friend request to yourself".into()));
    }

    let target_row = sqlx::query(
        "SELECT parental_controls_enabled FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(target_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let parental: bool = target_row.try_get("parental_controls_enabled").unwrap_or(false);

    // Requester display name for the notification detail
    let requester_name: String = sqlx::query(
        "SELECT display_name FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .and_then(|r| r.try_get::<String, _>("display_name").ok())
    .unwrap_or_else(|| "Unknown".into());

    if parental {
        let req_row = sqlx::query(
            "INSERT INTO pending_friend_requests (child_user_id, requester_id, requester_name)
             VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING
             RETURNING id",
        )
        .bind(target_id)
        .bind(auth.user_id)
        .bind(&requester_name)
        .fetch_optional(state.db.pool())
        .await?;

        if let Some(r) = req_row {
            let req_id: Uuid = r.try_get("id")?;
            notify_parent(target_id, "friend_request", req_id, &requester_name, &state).await;
        }

        return Ok((
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "status": "pending_parental_approval" })),
        ));
    }

    // Standard flow — insert friendship in pending state (lower UUID first)
    let (uid1, uid2) = if auth.user_id < target_id {
        (auth.user_id, target_id)
    } else {
        (target_id, auth.user_id)
    };

    sqlx::query(
        "INSERT INTO friendships (user_id_1, user_id_2, status) VALUES ($1, $2, 'pending')
         ON CONFLICT (user_id_1, user_id_2) DO NOTHING",
    )
    .bind(uid1)
    .bind(uid2)
    .execute(state.db.pool())
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "status": "pending" })),
    ))
}

// ─── Activity feed ────────────────────────────────────────────────────────────

/// GET /api/v1/users/me/feed — recent hype moments from followed users.
async fn get_feed(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT hm.id, hm.user_id, hm.message_id, hm.type, hm.pinned_at,
                u.username, u.display_name, u.avatar_url
         FROM hype_moments hm
         JOIN followers f ON f.following_id = hm.user_id AND f.follower_id = $1
         JOIN users u ON u.id = hm.user_id
         WHERE u.deleted_at IS NULL
         ORDER BY hm.pinned_at DESC
         LIMIT 40",
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({
            "id":         r.try_get::<Uuid, _>("id").ok(),
            "user_id":    r.try_get::<Uuid, _>("user_id").ok(),
            "message_id": r.try_get::<Uuid, _>("message_id").ok(),
            "type":       r.try_get::<String, _>("type").unwrap_or_default(),
            "pinned_at":  r.try_get::<chrono::DateTime<chrono::Utc>, _>("pinned_at")
                            .ok().map(|t| t.to_rfc3339()),
            "author": {
                "username":     r.try_get::<String, _>("username").unwrap_or_default(),
                "display_name": r.try_get::<Option<String>, _>("display_name").ok().flatten(),
                "avatar_url":   r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
            },
        }))
        .collect();

    Ok(Json(serde_json::json!({ "items": items })))
}

// ─── Hype moments ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct HypeMomentInput {
    message_id: Uuid,
    #[serde(rename = "type")]
    moment_type: String,
}

/// POST /api/v1/users/me/hype-moments
async fn pin_hype_moment(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<HypeMomentInput>,
) -> AppResult<impl IntoResponse> {
    if !["yap", "clip", "text"].contains(&body.moment_type.as_str()) {
        return Err(AppError::BadRequest("type must be yap, clip, or text".into()));
    }

    // Max 9 hype moments per profile
    let count: i64 = sqlx::query("SELECT COUNT(*) FROM hype_moments WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_one(state.db.pool())
        .await
        .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
        .unwrap_or(0);

    if count >= 9 {
        return Err(AppError::BadRequest(
            "Maximum 9 hype moments — remove one first".into(),
        ));
    }

    let row = sqlx::query(
        "INSERT INTO hype_moments (user_id, message_id, type) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(body.message_id)
    .bind(&body.moment_type)
    .fetch_one(state.db.pool())
    .await?;

    let id: Uuid = row.try_get("id")?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

/// GET /api/v1/users/by/:username/hype-moments
async fn get_hype_moments(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> AppResult<impl IntoResponse> {
    let target_id = resolve_user(&username, &state).await?;

    let rows = sqlx::query(
        "SELECT id, message_id, type, pinned_at FROM hype_moments
         WHERE user_id = $1 ORDER BY pinned_at DESC LIMIT 100",
    )
    .bind(target_id)
    .fetch_all(state.db.pool())
    .await?;

    let moments: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({
            "id":         r.try_get::<Uuid, _>("id").ok(),
            "message_id": r.try_get::<Uuid, _>("message_id").ok(),
            "type":       r.try_get::<String, _>("type").unwrap_or_default(),
            "pinned_at":  r.try_get::<chrono::DateTime<chrono::Utc>, _>("pinned_at")
                            .ok().map(|t| t.to_rfc3339()),
        }))
        .collect();

    Ok(Json(serde_json::json!({ "moments": moments })))
}

// ─── Settings: Profile update ─────────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateProfileInput {
    display_name: Option<String>,
    about_me: Option<String>,
    location: Option<String>,
    profile_theme_color: Option<String>,
    avatar_url: Option<String>,
    banner_url: Option<String>,
}

/// PATCH /api/v1/users/me
async fn update_profile(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<UpdateProfileInput>,
) -> AppResult<impl IntoResponse> {
    if let Some(ref v) = body.display_name {
        let trimmed = v.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_DISPLAY_NAME {
            return Err(AppError::BadRequest(
                format!("display_name must be 1–{MAX_DISPLAY_NAME} characters"),
            ));
        }
    }
    if let Some(ref v) = body.about_me {
        if v.len() > MAX_ABOUT_ME {
            return Err(AppError::BadRequest(
                format!("about_me must be at most {MAX_ABOUT_ME} characters"),
            ));
        }
    }
    if let Some(ref v) = body.location {
        if v.len() > MAX_LOCATION {
            return Err(AppError::BadRequest(
                format!("location must be at most {MAX_LOCATION} characters"),
            ));
        }
    }
    if let Some(ref v) = body.profile_theme_color {
        if !HEX_COLOR_REGEX.is_match(v) {
            return Err(AppError::BadRequest(
                "profile_theme_color must be a hex color like #7C3AED".into(),
            ));
        }
    }
    for (field_name, url_opt) in &[
        ("avatar_url", &body.avatar_url),
        ("banner_url", &body.banner_url),
    ] {
        if let Some(ref url) = url_opt {
            if !HTTPS_URL_REGEX.is_match(url) {
                return Err(AppError::BadRequest(
                    format!("{field_name} must start with https://"),
                ));
            }
        }
    }

    sqlx::query(
        "UPDATE users SET
            display_name        = COALESCE($2, display_name),
            about_me            = COALESCE($3, about_me),
            location            = COALESCE($4, location),
            profile_theme_color = COALESCE($5, profile_theme_color),
            avatar_url          = COALESCE($6, avatar_url),
            banner_url          = COALESCE($7, banner_url)
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .bind(body.display_name.as_deref())
    .bind(body.about_me.as_deref())
    .bind(body.location.as_deref())
    .bind(body.profile_theme_color.as_deref())
    .bind(body.avatar_url.as_deref())
    .bind(body.banner_url.as_deref())
    .execute(state.db.pool())
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ─── Settings: Username change ────────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeUsernameInput {
    username: String,
}

/// PATCH /api/v1/users/me/username
async fn change_username(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ChangeUsernameInput>,
) -> AppResult<impl IntoResponse> {
    let new_username = body.username.trim().to_string();

    if !USERNAME_REGEX.is_match(&new_username) {
        return Err(AppError::BadRequest(
            "username must be 3–32 characters: letters, numbers, underscores only".into(),
        ));
    }

    // Fetch current username + last change timestamp
    let row = sqlx::query(
        "SELECT username, username_changed_at FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(AppError::Unauthorized)?;

    let current_username: String = row.try_get("username")?;
    if current_username == new_username {
        return Ok(StatusCode::NO_CONTENT);
    }

    let last_changed: Option<chrono::DateTime<Utc>> =
        row.try_get("username_changed_at").unwrap_or(None);
    if let Some(t) = last_changed {
        let days_since = (Utc::now() - t).num_days();
        if days_since < USERNAME_COOLDOWN_DAYS {
            let days_remaining = USERNAME_COOLDOWN_DAYS - days_since;
            return Err(AppError::Conflict(format!(
                "Username can only be changed once every {USERNAME_COOLDOWN_DAYS} days \
                 ({days_remaining} days remaining)",
            )));
        }
    }

    // Check uniqueness
    let taken = sqlx::query(
        "SELECT 1 FROM users WHERE username = $1 AND id != $2 AND deleted_at IS NULL",
    )
    .bind(&new_username)
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .is_some();

    if taken {
        return Err(AppError::Conflict(format!(
            "Username @{new_username} is already taken",
        )));
    }

    sqlx::query(
        "UPDATE users SET username = $2, username_changed_at = NOW() WHERE id = $1",
    )
    .bind(auth.user_id)
    .bind(&new_username)
    .execute(state.db.pool())
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ─── Settings: Privacy ────────────────────────────────────────────────────────

/// GET /api/v1/users/me/privacy
async fn get_privacy(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query(
        "SELECT dm_permission, friend_request_permission, search_visible, show_last_seen
         FROM user_privacy_settings WHERE user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?;

    // Return defaults if no row exists yet
    let settings = match row {
        Some(r) => serde_json::json!({
            "dm_permission":             r.try_get::<String, _>("dm_permission").unwrap_or_else(|_| "everyone".into()),
            "friend_request_permission": r.try_get::<String, _>("friend_request_permission").unwrap_or_else(|_| "everyone".into()),
            "search_visible":            r.try_get::<bool, _>("search_visible").unwrap_or(true),
            "show_last_seen":            r.try_get::<bool, _>("show_last_seen").unwrap_or(true),
        }),
        None => serde_json::json!({
            "dm_permission":             "everyone",
            "friend_request_permission": "everyone",
            "search_visible":            true,
            "show_last_seen":            true,
        }),
    };

    Ok(Json(settings))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdatePrivacyInput {
    dm_permission: Option<String>,
    friend_request_permission: Option<String>,
    search_visible: Option<bool>,
    show_last_seen: Option<bool>,
}

/// PATCH /api/v1/users/me/privacy
async fn update_privacy(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<UpdatePrivacyInput>,
) -> AppResult<impl IntoResponse> {
    let valid_dm = ["everyone", "friends", "nobody"];
    let valid_fr = ["everyone", "friends_of_friends", "nobody"];

    if let Some(ref v) = body.dm_permission {
        if !valid_dm.contains(&v.as_str()) {
            return Err(AppError::BadRequest(
                "dm_permission must be one of: everyone, friends, nobody".into(),
            ));
        }
    }
    if let Some(ref v) = body.friend_request_permission {
        if !valid_fr.contains(&v.as_str()) {
            return Err(AppError::BadRequest(
                "friend_request_permission must be one of: everyone, friends_of_friends, nobody".into(),
            ));
        }
    }

    sqlx::query(
        "INSERT INTO user_privacy_settings
            (user_id, dm_permission, friend_request_permission, search_visible, show_last_seen, updated_at)
         VALUES ($1, $2, $3, $4, $5, NOW())
         ON CONFLICT (user_id) DO UPDATE SET
            dm_permission             = COALESCE($2, user_privacy_settings.dm_permission),
            friend_request_permission = COALESCE($3, user_privacy_settings.friend_request_permission),
            search_visible            = COALESCE($4, user_privacy_settings.search_visible),
            show_last_seen            = COALESCE($5, user_privacy_settings.show_last_seen),
            updated_at                = NOW()",
    )
    .bind(auth.user_id)
    .bind(body.dm_permission.as_deref())
    .bind(body.friend_request_permission.as_deref())
    .bind(body.search_visible)
    .bind(body.show_last_seen)
    .execute(state.db.pool())
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ─── Account: GDPR export ─────────────────────────────────────────────────────

/// GET /api/v1/account/data-export
///
/// Returns a JSON document with the user's non-encrypted account data.
/// Message *ciphertext* is intentionally excluded — it is client-side only.
async fn gdpr_export(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    // Profile
    let profile_row = sqlx::query(
        "SELECT username, display_name, email, about_me, location,
                profile_theme_color, account_type, created_at
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(AppError::Unauthorized)?;

    // Friendships
    let friend_rows = sqlx::query(
        "SELECT u.username, u.display_name, f.status, f.created_at
         FROM friendships f
         JOIN users u ON u.id = CASE WHEN f.user_id_1 = $1 THEN f.user_id_2 ELSE f.user_id_1 END
         WHERE (f.user_id_1 = $1 OR f.user_id_2 = $1) AND f.status = 'accepted'
           AND u.deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    // Server memberships
    let server_rows = sqlx::query(
        "SELECT s.name, s.slug, sm.role, sm.joined_at
         FROM server_memberships sm
         JOIN servers s ON s.id = sm.server_id
         WHERE sm.user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    // Message metadata only — NO ciphertext
    let message_meta_rows = sqlx::query(
        "SELECT id, channel_id, conversation_id, message_type, created_at
         FROM messages
         WHERE sender_id = $1 AND deleted_at IS NULL
         ORDER BY created_at DESC
         LIMIT 1000",
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let export = serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "note": "Message content is end-to-end encrypted and not held by Yapper servers.",
        "profile": {
            "username":           profile_row.try_get::<String, _>("username").unwrap_or_default(),
            "display_name":       profile_row.try_get::<String, _>("display_name").unwrap_or_default(),
            "email":              profile_row.try_get::<String, _>("email").unwrap_or_default(),
            "about_me":           profile_row.try_get::<Option<String>, _>("about_me").ok().flatten(),
            "location":           profile_row.try_get::<Option<String>, _>("location").ok().flatten(),
            "profile_theme_color":profile_row.try_get::<Option<String>, _>("profile_theme_color").ok().flatten(),
            "account_type":       profile_row.try_get::<String, _>("account_type").unwrap_or_default(),
            "created_at":         profile_row.try_get::<chrono::DateTime<Utc>, _>("created_at")
                                    .ok().map(|t| t.to_rfc3339()),
        },
        "friends": friend_rows.iter().map(|r| serde_json::json!({
            "username":     r.try_get::<String, _>("username").unwrap_or_default(),
            "display_name": r.try_get::<String, _>("display_name").unwrap_or_default(),
            "since":        r.try_get::<chrono::DateTime<Utc>, _>("created_at")
                              .ok().map(|t| t.to_rfc3339()),
        })).collect::<Vec<_>>(),
        "servers": server_rows.iter().map(|r| serde_json::json!({
            "name":      r.try_get::<String, _>("name").unwrap_or_default(),
            "slug":      r.try_get::<String, _>("slug").unwrap_or_default(),
            "role":      r.try_get::<String, _>("role").unwrap_or_default(),
            "joined_at": r.try_get::<chrono::DateTime<Utc>, _>("joined_at")
                           .ok().map(|t| t.to_rfc3339()),
        })).collect::<Vec<_>>(),
        "message_metadata": message_meta_rows.iter().map(|r| serde_json::json!({
            "id":              r.try_get::<Uuid, _>("id").ok(),
            "channel_id":      r.try_get::<Option<Uuid>, _>("channel_id").ok().flatten(),
            "conversation_id": r.try_get::<Option<Uuid>, _>("conversation_id").ok().flatten(),
            "type":            r.try_get::<String, _>("message_type").unwrap_or_default(),
            "sent_at":         r.try_get::<chrono::DateTime<Utc>, _>("created_at")
                                 .ok().map(|t| t.to_rfc3339()),
        })).collect::<Vec<_>>(),
    });

    let json_bytes = serde_json::to_vec_pretty(&export)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Serialization failed: {e}")))?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"yapper-data-export.json\""),
        ],
        json_bytes,
    ))
}

// ─── Account: Soft delete ─────────────────────────────────────────────────────

/// DELETE /api/v1/account
///
/// Soft-deletes the account. The user immediately cannot log in.
/// A background job (scheduled separately) handles full PII purge after 30 days.
async fn delete_account(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    // Soft-delete the user
    let result = sqlx::query(
        "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .execute(state.db.pool())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Account not found or already deleted".into()));
    }

    // Invalidate all sessions immediately
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(state.db.pool())
        .await
        .map_err(|e| {
            tracing::error!("Failed to purge sessions on account delete: {e}");
            e
        })?;

    tracing::info!(user_id = %auth.user_id, "Account soft-deleted");
    Ok(StatusCode::NO_CONTENT)
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

async fn resolve_user(username: &str, state: &AppState) -> AppResult<Uuid> {
    sqlx::query("SELECT id FROM users WHERE username = $1 AND deleted_at IS NULL")
        .bind(username)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User @{username} not found")))?
        .try_get("id")
        .map_err(AppError::Database)
}

/// Persist a parent_notification row and push a real-time WS event to the parent.
pub async fn notify_parent(
    child_id: Uuid,
    notification_type: &str,
    reference_id: Uuid,
    detail: &str,
    state: &AppState,
) {
    let parent = sqlx::query(
        "SELECT parent_user_id FROM parent_child_relationships WHERE child_user_id = $1 LIMIT 1",
    )
    .bind(child_id)
    .fetch_optional(state.db.pool())
    .await;

    let parent_id: Uuid = match parent {
        Ok(Some(r)) => match r.try_get("parent_user_id") {
            Ok(id) => id,
            Err(_) => return,
        },
        _ => return,
    };

    let _ = sqlx::query(
        "INSERT INTO parent_notifications (parent_user_id, child_user_id, type, reference_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(parent_id)
    .bind(child_id)
    .bind(notification_type)
    .bind(reference_id)
    .execute(state.db.pool())
    .await;

    let payload = serde_json::json!({
        "type":         notification_type,
        "child_id":     child_id,
        "reference_id": reference_id,
        "detail":       detail,
    });
    state
        .hub
        .send_to_user(&parent_id, crate::hub::WsOutbound::ParentNotification { payload });
}
