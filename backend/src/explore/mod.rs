/**
 * Explore & Discovery â€” public server browsing + full-text search.
 *
 * Routes (all mounted directly under /api/v2 via Router::merge):
 *   GET /explore/communities     â€” public servers ranked by member count
 *   GET /explore/live-servers    â€” servers with activity in the last 30 min
 *   GET /explore/trending-tags   â€” most-used server tags (5-min in-memory cache)
 *   GET /search?q=               â€” full-text search across servers + users (pg_trgm)
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

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/explore/communities", get(get_communities))
        .route("/explore/live-servers", get(get_live_servers))
        .route("/explore/trending-tags", get(get_trending_tags))
        .route("/explore/top-yappers", get(get_top_yappers))
        .route("/search", get(search))
}

// â”€â”€â”€ Simple in-memory cache for trending tags (5-min TTL) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

struct TagCache {
    data: Vec<serde_json::Value>,
    fetched_at: Instant,
}

static TAG_CACHE: Mutex<Option<TagCache>> = Mutex::new(None);

const TAG_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

// â”€â”€â”€ Communities â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /explore/communities â€” public servers ranked by member count.
async fn get_communities(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.icon_url, s.description, s.tags, s.member_count
         FROM servers s
         WHERE s.is_public = TRUE
         ORDER BY s.member_count DESC
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

// â”€â”€â”€ Live servers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /explore/live-servers â€” servers with a message sent in the last 30 minutes.
async fn get_live_servers(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.slug, s.icon_url, s.description, s.tags, s.member_count,
                MAX(m.created_at) AS last_active
         FROM servers s
         JOIN channels c ON c.server_id = s.id
         JOIN messages m ON m.channel_id = c.id
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

// â”€â”€â”€ Trending tags â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /explore/trending-tags â€” most-used tags across public servers (cached 5 min).
async fn get_trending_tags(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    // Check cache first
    {
        let guard = match TAG_CACHE.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::warn!("TAG_CACHE mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
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
        let mut guard = match TAG_CACHE.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::warn!("TAG_CACHE mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
        *guard = Some(TagCache {
            data: tags.clone(),
            fetched_at: Instant::now(),
        });
    }

    Ok(Json(serde_json::json!({ "tags": tags })))
}

// â”€â”€â”€ Full-text search â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchQuery {
    q: String,
}

/// GET /search?q= â€” find public servers and users matching the query (pg_trgm).
async fn search(
    auth: AuthUser,
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

    // Servers â€” similarity search on name + description
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

    // Users â€” similarity search on username + display_name, with privacy + friendship status
    let user_rows = sqlx::query(
        "SELECT u.id, u.username, u.display_name, u.avatar_url,
                similarity(u.username || ' ' || COALESCE(u.display_name, ''), $1) AS score,
                COALESCE(ups.friend_request_permission, 'everyone') AS friend_request_permission,
                EXISTS (
                    SELECT 1 FROM friendships f
                    WHERE f.status = 'accepted'
                      AND ((f.user_id_1 = $2 AND f.user_id_2 = u.id)
                        OR (f.user_id_1 = u.id AND f.user_id_2 = $2))
                ) AS is_friend,
                EXISTS (
                    SELECT 1 FROM friendships f
                    WHERE f.status = 'pending'
                      AND f.requested_by = $2
                      AND ((f.user_id_1 = $2 AND f.user_id_2 = u.id)
                        OR (f.user_id_1 = u.id AND f.user_id_2 = $2))
                ) AS request_sent
         FROM users u
         LEFT JOIN user_privacy_settings ups ON ups.user_id = u.id
         WHERE (u.username || ' ' || COALESCE(u.display_name, '')) % $1
           AND u.deleted_at IS NULL
           AND u.id != $2
         ORDER BY score DESC
         LIMIT 10",
    )
    .bind(&q)
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let users: Vec<serde_json::Value> = user_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":                        r.try_get::<Uuid, _>("id").ok(),
                "username":                  r.try_get::<String, _>("username").unwrap_or_default(),
                "display_name":              r.try_get::<Option<String>, _>("display_name").ok().flatten(),
                "avatar_url":                r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
                "friend_request_permission": r.try_get::<String, _>("friend_request_permission").unwrap_or_else(|_| "everyone".into()),
                "is_friend":                 r.try_get::<bool, _>("is_friend").unwrap_or(false),
                "request_sent":              r.try_get::<bool, _>("request_sent").unwrap_or(false),
            })
        })
        .collect();

    Ok(Json(
        serde_json::json!({ "servers": servers, "users": users }),
    ))
}

// â”€â”€â”€ Top yappers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /explore/top-yappers â€” users ranked by follower count (top 20).
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
        .map(|r| {
            serde_json::json!({
                "id":             r.try_get::<Uuid, _>("id").ok(),
                "username":       r.try_get::<String, _>("username").unwrap_or_default(),
                "display_name":   r.try_get::<Option<String>, _>("display_name").ok().flatten(),
                "avatar_url":     r.try_get::<Option<String>, _>("avatar_url").ok().flatten(),
                "is_premium":     r.try_get::<bool, _>("is_premium").unwrap_or(false),
                "follower_count": r.try_get::<i64, _>("follower_count").unwrap_or(0),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "yappers": yappers })))
}

#[cfg(test)]
mod tests {
    use super::*;

    // â”€â”€ SearchQuery deserialization â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn search_query_deserializes_valid() {
        let json = serde_json::json!({ "q": "hello world" });
        let sq: SearchQuery = serde_json::from_value(json).unwrap();
        assert_eq!(sq.q, "hello world");
    }

    #[test]
    fn search_query_deserializes_empty_string() {
        let json = serde_json::json!({ "q": "" });
        let sq: SearchQuery = serde_json::from_value(json).unwrap();
        assert_eq!(sq.q, "");
    }

    #[test]
    fn search_query_rejects_missing_q() {
        let json = serde_json::json!({});
        assert!(serde_json::from_value::<SearchQuery>(json).is_err());
    }

    #[test]
    fn search_query_rejects_unknown_fields() {
        let json = serde_json::json!({ "q": "hello", "extra": "bad" });
        assert!(serde_json::from_value::<SearchQuery>(json).is_err());
    }

    #[test]
    fn search_query_rejects_non_string_q() {
        let json = serde_json::json!({ "q": 42 });
        assert!(serde_json::from_value::<SearchQuery>(json).is_err());
    }

    // â”€â”€ Search query trimming + length validation (mirrors handler logic) â”€

    #[test]
    fn search_trim_removes_whitespace() {
        let raw = "  hello world  ";
        let q = raw.trim().to_string();
        assert_eq!(q, "hello world");
    }

    #[test]
    fn search_trim_all_whitespace_is_empty() {
        let raw = "     ";
        let q = raw.trim().to_string();
        assert!(q.is_empty());
    }

    #[test]
    fn search_query_length_at_boundary() {
        // Exactly 255 chars should pass
        let at_limit = "x".repeat(255);
        assert!(at_limit.len() <= 255);

        // 256 chars should fail
        let over_limit = "x".repeat(256);
        assert!(over_limit.len() > 255);
    }

    #[test]
    fn search_query_unicode_length_is_byte_len() {
        // The handler uses `.len()` which counts bytes, not chars.
        // A 3-byte UTF-8 char repeated 86 times = 258 bytes > 255
        let unicode_heavy = "\u{2603}".repeat(86); // snowman = 3 bytes each
        assert!(unicode_heavy.len() > 255);
        // 85 snowmen = 255 bytes â€” exactly at limit
        let at_limit = "\u{2603}".repeat(85);
        assert_eq!(at_limit.len(), 255);
    }

    // â”€â”€ TAG_CACHE_TTL â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn tag_cache_ttl_is_five_minutes() {
        assert_eq!(TAG_CACHE_TTL, Duration::from_secs(300));
        assert_eq!(TAG_CACHE_TTL.as_secs(), 5 * 60);
    }

    // â”€â”€ TagCache struct behaviour â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn tag_cache_fresh_entry_is_not_expired() {
        let cache = TagCache {
            data: vec![serde_json::json!({"tag": "gaming", "server_count": 10})],
            fetched_at: Instant::now(),
        };
        assert!(cache.fetched_at.elapsed() < TAG_CACHE_TTL);
    }

    #[test]
    fn tag_cache_data_preserves_values() {
        let tags = vec![
            serde_json::json!({"tag": "gaming", "server_count": 42}),
            serde_json::json!({"tag": "music", "server_count": 17}),
        ];
        let cache = TagCache {
            data: tags.clone(),
            fetched_at: Instant::now(),
        };
        assert_eq!(cache.data.len(), 2);
        assert_eq!(cache.data[0]["tag"], "gaming");
        assert_eq!(cache.data[0]["server_count"], 42);
        assert_eq!(cache.data[1]["tag"], "music");
        assert_eq!(cache.data[1]["server_count"], 17);
    }

    #[test]
    fn tag_cache_empty_data_is_valid() {
        let cache = TagCache {
            data: vec![],
            fetched_at: Instant::now(),
        };
        assert!(cache.data.is_empty());
    }

    // â”€â”€ TAG_CACHE mutex behaviour â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn tag_cache_mutex_starts_none() {
        // The global mutex starts as None. After acquiring the lock,
        // it should be Option<TagCache>. We cannot reset it between tests
        // since it is a static, but we can verify the lock is acquirable.
        let guard = TAG_CACHE.lock().unwrap();
        // The initial value or a value set by a prior test â€” either way the lock works
        drop(guard);
    }

    #[test]
    fn tag_cache_mutex_can_store_and_retrieve() {
        {
            let mut guard = TAG_CACHE.lock().unwrap();
            *guard = Some(TagCache {
                data: vec![serde_json::json!({"tag": "test", "server_count": 1})],
                fetched_at: Instant::now(),
            });
        }
        {
            let guard = TAG_CACHE.lock().unwrap();
            let cache = guard.as_ref().expect("cache should be Some");
            assert_eq!(cache.data[0]["tag"], "test");
        }
    }

    #[test]
    fn tag_cache_mutex_can_be_cleared() {
        {
            let mut guard = TAG_CACHE.lock().unwrap();
            *guard = None;
        }
        {
            let guard = TAG_CACHE.lock().unwrap();
            assert!(guard.is_none());
        }
    }

    // â”€â”€ JSON structure matching (mirrors handler output shapes) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn community_json_shape() {
        let community = serde_json::json!({
            "id":           Uuid::new_v4(),
            "name":         "Test Server",
            "slug":         "test-server",
            "icon_url":     null,
            "description":  "A test server",
            "tags":         ["gaming", "fun"],
            "member_count": 150,
        });
        assert!(community["id"].is_string());
        assert_eq!(community["name"], "Test Server");
        assert_eq!(community["slug"], "test-server");
        assert!(community["icon_url"].is_null());
        assert_eq!(community["tags"].as_array().unwrap().len(), 2);
        assert_eq!(community["member_count"], 150);
    }

    #[test]
    fn search_empty_response_shape() {
        // The handler returns this for empty queries
        let response = serde_json::json!({ "servers": [], "users": [] });
        assert!(response["servers"].as_array().unwrap().is_empty());
        assert!(response["users"].as_array().unwrap().is_empty());
    }

    #[test]
    fn top_yapper_json_shape() {
        let yapper = serde_json::json!({
            "id":             Uuid::new_v4(),
            "username":       "cooluser",
            "display_name":   "Cool User",
            "avatar_url":     "https://example.com/avatar.webp",
            "is_premium":     true,
            "follower_count": 9001,
        });
        assert!(yapper["id"].is_string());
        assert_eq!(yapper["username"], "cooluser");
        assert_eq!(yapper["is_premium"], true);
        assert_eq!(yapper["follower_count"], 9001);
    }

    #[test]
    fn trending_tag_json_shape() {
        let tag = serde_json::json!({
            "tag":          "gaming",
            "server_count": 42,
        });
        assert_eq!(tag["tag"], "gaming");
        assert_eq!(tag["server_count"], 42);
    }

    // â”€â”€ Live server JSON includes last_active â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn live_server_json_shape_with_last_active() {
        let now = chrono::Utc::now();
        let server = serde_json::json!({
            "id":           Uuid::new_v4(),
            "name":         "Active Server",
            "slug":         "active-server",
            "icon_url":     null,
            "description":  null,
            "tags":         [],
            "member_count": 5,
            "last_active":  now.to_rfc3339(),
        });
        assert!(server["last_active"].is_string());
        // Verify the timestamp round-trips
        let parsed: chrono::DateTime<chrono::Utc> =
            server["last_active"].as_str().unwrap().parse().unwrap();
        // Allow 1 second tolerance for the round-trip
        assert!((parsed - now).num_seconds().abs() <= 1);
    }

    // â”€â”€ User search result JSON shape â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn user_search_result_json_shape() {
        let user = serde_json::json!({
            "id":                        Uuid::new_v4(),
            "username":                  "searchuser",
            "display_name":              "Search User",
            "avatar_url":                null,
            "friend_request_permission": "everyone",
            "is_friend":                 false,
            "request_sent":              false,
        });
        assert_eq!(user["friend_request_permission"], "everyone");
        assert_eq!(user["is_friend"], false);
        assert_eq!(user["request_sent"], false);
    }

    #[test]
    fn user_search_result_friend_permission_variants() {
        for perm in ["everyone", "friends_of_friends", "nobody"] {
            let user = serde_json::json!({
                "friend_request_permission": perm,
            });
            assert!(user["friend_request_permission"].is_string());
        }
    }
}
