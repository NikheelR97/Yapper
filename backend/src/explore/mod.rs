/**
 * Explore & Discovery — public server browsing + full-text search.
 *
 * Routes (all mounted directly under /api/v1 via Router::merge):
 *   GET /explore/communities     — public servers ranked by member count
 *   GET /explore/live-servers    — servers with activity in the last 30 min
 *   GET /explore/trending-tags   — most-used server tags (5-min in-memory cache)
 *   GET /search?q=               — full-text search across servers + users (pg_trgm)
 */

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::Row;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

use crate::{auth::AuthUser, error::{AppError, AppResult}, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/explore/communities", get(get_communities))
        .route("/explore/live-servers", get(get_live_servers))
        .route("/explore/trending-tags", get(get_trending_tags))
        .route("/explore/top-yappers", get(get_top_yappers))
        .route("/search", get(search))
}

// ─── Simple in-memory cache for trending tags (5-min TTL) ────────────────────

struct TagCache {
    data: Vec<serde_json::Value>,
    fetched_at: Instant,
}

static TAG_CACHE: Mutex<Option<TagCache>> = Mutex::new(None);

const TAG_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

// ─── Communities ──────────────────────────────────────────────────────────────

/// GET /explore/communities — public servers ranked by member count.
async fn get_communities(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.icon_url, s.description, s.tags,
                COUNT(sm.user_id) AS member_count
         FROM servers s
         LEFT JOIN server_memberships sm ON sm.server_id = s.id
         WHERE s.is_public = TRUE
         GROUP BY s.id
         ORDER BY member_count DESC
         LIMIT 50",
    )
    .fetch_all(state.db.pool())
    .await?;

    let communities: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":           r.try_get::<Uuid, _>("id").ok(),
                "name":         r.try_get::<String, _>("name").unwrap_or_default(),
                "slug":         r.try_get::<String, _>("slug").unwrap_or_default(),
                "icon_url":     r.try_get::<Option<String>, _>("icon_url").ok().flatten(),
                "description":  r.try_get::<Option<String>, _>("description").ok().flatten(),
                "tags":         r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
                "member_count": r.try_get::<i64, _>("member_count").unwrap_or(0),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "communities": communities })))
}

// ─── Live servers ─────────────────────────────────────────────────────────────

/// GET /explore/live-servers — servers with a message sent in the last 30 minutes.
async fn get_live_servers(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.icon_url, s.description, s.tags,
                COUNT(DISTINCT sm.user_id) AS member_count,
                MAX(m.created_at) AS last_active
         FROM servers s
         JOIN channels c ON c.server_id = s.id
         JOIN messages m ON m.channel_id = c.id
         LEFT JOIN server_memberships sm ON sm.server_id = s.id
         WHERE s.is_public = TRUE
           AND m.created_at > NOW() - INTERVAL '30 minutes'
           AND m.deleted_at IS NULL
         GROUP BY s.id
         ORDER BY last_active DESC
         LIMIT 20",
    )
    .fetch_all(state.db.pool())
    .await?;

    let servers: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":           r.try_get::<Uuid, _>("id").ok(),
                "name":         r.try_get::<String, _>("name").unwrap_or_default(),
                "slug":         r.try_get::<String, _>("slug").unwrap_or_default(),
                "icon_url":     r.try_get::<Option<String>, _>("icon_url").ok().flatten(),
                "description":  r.try_get::<Option<String>, _>("description").ok().flatten(),
                "tags":         r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
                "member_count": r.try_get::<i64, _>("member_count").unwrap_or(0),
                "last_active":  r.try_get::<chrono::DateTime<chrono::Utc>, _>("last_active")
                                  .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "servers": servers })))
}

// ─── Trending tags ────────────────────────────────────────────────────────────

/// GET /explore/trending-tags — most-used tags across public servers (cached 5 min).
async fn get_trending_tags(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    // Check cache first
    {
        let guard = TAG_CACHE.lock().unwrap();
        if let Some(ref cache) = *guard {
            if cache.fetched_at.elapsed() < TAG_CACHE_TTL {
                return Ok(Json(serde_json::json!({ "tags": cache.data })));
            }
        }
    }

    let rows = sqlx::query(
        "SELECT unnest(tags) AS tag, COUNT(*) AS server_count
         FROM servers
         WHERE is_public = TRUE AND cardinality(tags) > 0
         GROUP BY tag
         ORDER BY server_count DESC
         LIMIT 30",
    )
    .fetch_all(state.db.pool())
    .await?;

    let tags: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "tag":          r.try_get::<String, _>("tag").unwrap_or_default(),
                "server_count": r.try_get::<i64, _>("server_count").unwrap_or(0),
            })
        })
        .collect();

    // Update cache
    {
        let mut guard = TAG_CACHE.lock().unwrap();
        *guard = Some(TagCache { data: tags.clone(), fetched_at: Instant::now() });
    }

    Ok(Json(serde_json::json!({ "tags": tags })))
}

// ─── Full-text search ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

/// GET /search?q= — find public servers and users matching the query (pg_trgm).
async fn search(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> AppResult<impl IntoResponse> {
    let q = params.q.trim().to_string();
    if q.is_empty() {
        return Ok(Json(serde_json::json!({ "servers": [], "users": [] })));
    }
    if q.len() > 255 {
        return Err(AppError::BadRequest("Search query too long".into()));
    }

    // Servers — similarity search on name + description
    let server_rows = sqlx::query(
        "SELECT id, name, slug, icon_url, description, tags,
                similarity(name || ' ' || COALESCE(description, ''), $1) AS score
         FROM servers
         WHERE is_public = TRUE
           AND (name || ' ' || COALESCE(description, '')) % $1
         ORDER BY score DESC
         LIMIT 20",
    )
    .bind(&q)
    .fetch_all(state.db.pool())
    .await?;

    let servers: Vec<serde_json::Value> = server_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":          r.try_get::<Uuid, _>("id").ok(),
                "name":        r.try_get::<String, _>("name").unwrap_or_default(),
                "slug":        r.try_get::<String, _>("slug").unwrap_or_default(),
                "icon_url":    r.try_get::<Option<String>, _>("icon_url").ok().flatten(),
                "description": r.try_get::<Option<String>, _>("description").ok().flatten(),
                "tags":        r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
            })
        })
        .collect();

    // Users — similarity search on username + display_name
    let user_rows = sqlx::query(
        "SELECT id, username, display_name, avatar_url,
                similarity(username || ' ' || COALESCE(display_name, ''), $1) AS score
         FROM users
         WHERE (username || ' ' || COALESCE(display_name, '')) % $1
           AND deleted_at IS NULL
         ORDER BY score DESC
         LIMIT 10",
    )
    .bind(&q)
    .fetch_all(state.db.pool())
    .await?;

    let users: Vec<serde_json::Value> = user_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":           r.try_get::<Uuid, _>("id").ok(),
                "username":     r.try_get::<String, _>("username").unwrap_or_default(),
                "display_name": r.try_get::<Option<String>, _>("display_name").ok().flatten(),
                "avatar_url":   r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "servers": servers, "users": users })))
}

// ─── Top yappers ─────────────────────────────────────────────────────────────

/// GET /explore/top-yappers — users ranked by follower count (top 20).
async fn get_top_yappers(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT u.id, u.username, u.display_name, u.avatar_url, u.is_premium,
                COUNT(f.follower_id) AS follower_count
         FROM users u
         LEFT JOIN followers f ON f.following_id = u.id
         WHERE u.deleted_at IS NULL
           AND u.account_type != 'bot'
         GROUP BY u.id
         ORDER BY follower_count DESC
         LIMIT 20",
    )
    .fetch_all(state.db.pool())
    .await?;

    let yappers: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({
            "id":             r.try_get::<Uuid, _>("id").ok(),
            "username":       r.try_get::<String, _>("username").unwrap_or_default(),
            "display_name":   r.try_get::<Option<String>, _>("display_name").ok().flatten(),
            "avatar_url":     r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
            "is_premium":     r.try_get::<bool, _>("is_premium").unwrap_or(false),
            "follower_count": r.try_get::<i64, _>("follower_count").unwrap_or(0),
        }))
        .collect();

    Ok(Json(serde_json::json!({ "yappers": yappers })))
}
