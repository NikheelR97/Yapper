use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    hub::WsOutbound,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Music state
        .route("/canvas/servers/:server_id/music", patch(patch_music))
        // Full canvas state (music + active polls)
        .route("/canvas/servers/:server_id", get(get_canvas))
        // Recent clip metadata for the clips carousel
        .route("/canvas/servers/:server_id/clips", get(get_clips))
        // Polls
        .route("/canvas/channels/:channel_id/polls", post(create_poll))
        .route("/canvas/polls/:poll_id/vote", post(vote_poll))
}

// ─── Music state ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct MusicInput {
    artist: String,
    title: String,
    source_url: Option<String>,
    album_art_url: Option<String>,
}

async fn patch_music(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<MusicInput>,
) -> AppResult<impl IntoResponse> {
    require_member(server_id, auth.user_id, &state).await?;

    sqlx::query(
        "INSERT INTO canvas_music_state
            (server_id, artist, title, source_url, album_art_url, set_by_user_id, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW())
         ON CONFLICT (server_id) DO UPDATE SET
            artist        = EXCLUDED.artist,
            title         = EXCLUDED.title,
            source_url    = EXCLUDED.source_url,
            album_art_url = EXCLUDED.album_art_url,
            set_by_user_id = EXCLUDED.set_by_user_id,
            updated_at    = NOW()",
    )
    .bind(server_id)
    .bind(&body.artist)
    .bind(&body.title)
    .bind(&body.source_url)
    .bind(&body.album_art_url)
    .bind(auth.user_id)
    .execute(state.db.pool())
    .await?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "music_update",
            "server_id": server_id,
            "artist": body.artist,
            "title": body.title,
            "source_url": body.source_url,
            "album_art_url": body.album_art_url,
            "set_by": auth.user_id,
        }),
        &state,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ─── Full canvas state ────────────────────────────────────────────────────────

async fn get_canvas(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    require_member(server_id, auth.user_id, &state).await?;

    let music = sqlx::query(
        "SELECT artist, title, source_url, album_art_url, set_by_user_id, updated_at
         FROM canvas_music_state WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?
    .map(|r| {
        serde_json::json!({
            "artist":        r.try_get::<String, _>("artist").unwrap_or_default(),
            "title":         r.try_get::<String, _>("title").unwrap_or_default(),
            "source_url":    r.try_get::<Option<String>, _>("source_url").ok().flatten(),
            "album_art_url": r.try_get::<Option<String>, _>("album_art_url").ok().flatten(),
            "set_by":        r.try_get::<Uuid, _>("set_by_user_id").ok(),
            "updated_at":    r.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                               .ok().map(|t| t.to_rfc3339()),
        })
    });

    // Active polls in any channel of this server (not expired, most recent first)
    let poll_rows = sqlx::query(
        "SELECT p.id, p.channel_id, p.question, p.options, p.ends_at, p.created_at,
                (SELECT jsonb_object_agg(v.option_index::text, v.cnt)
                 FROM (SELECT option_index, COUNT(*) AS cnt
                       FROM poll_votes WHERE poll_id = p.id
                       GROUP BY option_index) v) AS vote_counts
         FROM polls p
         JOIN channels c ON c.id = p.channel_id
         WHERE c.server_id = $1
           AND (p.ends_at IS NULL OR p.ends_at > NOW())
         ORDER BY p.created_at DESC
         LIMIT 10",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    let polls: Vec<serde_json::Value> = poll_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":          r.try_get::<Uuid, _>("id").ok(),
                "channel_id":  r.try_get::<Uuid, _>("channel_id").ok(),
                "question":    r.try_get::<String, _>("question").unwrap_or_default(),
                "options":     r.try_get::<serde_json::Value, _>("options")
                                 .unwrap_or(serde_json::Value::Array(vec![])),
                "vote_counts": r.try_get::<Option<serde_json::Value>, _>("vote_counts")
                                 .ok().flatten().unwrap_or(serde_json::json!({})),
                "ends_at":     r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("ends_at")
                                 .ok().flatten().map(|t| t.to_rfc3339()),
                "created_at":  r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                 .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "music": music, "polls": polls })))
}

// ─── Clips carousel ───────────────────────────────────────────────────────────

async fn get_clips(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    require_member(server_id, auth.user_id, &state).await?;

    // Return encrypted ciphertext — client decrypts using keys embedded in Signal payload
    let rows = sqlx::query(
        "SELECT m.id, m.sender_id, m.ciphertext, m.created_at, c.id AS channel_id
         FROM messages m
         JOIN channels c ON c.id = m.channel_id
         WHERE c.server_id = $1
           AND m.message_type = 'clip'
           AND m.deleted_at IS NULL
           AND m.ciphertext IS NOT NULL
         ORDER BY m.created_at DESC
         LIMIT 20",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    let clips: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let ciphertext = r
                .try_get::<Vec<u8>, _>("ciphertext")
                .ok()
                .map(|b| BASE64.encode(&b));
            serde_json::json!({
                "id":         r.try_get::<Uuid, _>("id").ok(),
                "channel_id": r.try_get::<Uuid, _>("channel_id").ok(),
                "sender_id":  r.try_get::<Uuid, _>("sender_id").ok(),
                "ciphertext": ciphertext,
                "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "clips": clips })))
}

// ─── Polls ────────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct PollInput {
    question: String,
    /// Plain-text option labels (2–10)
    options: Vec<String>,
    /// Optional ISO-8601 expiry timestamp
    ends_at: Option<String>,
}

async fn create_poll(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<PollInput>,
) -> AppResult<impl IntoResponse> {
    let server_id = require_channel_member(channel_id, auth.user_id, &state).await?;

    if body.options.len() < 2 || body.options.len() > 10 {
        return Err(AppError::BadRequest("Poll must have 2–10 options".into()));
    }
    if body.question.len() > 500 {
        return Err(AppError::BadRequest("Poll question too long".into()));
    }
    for opt in &body.options {
        if opt.len() > 500 {
            return Err(AppError::BadRequest("Poll option too long".into()));
        }
    }

    let options: Vec<serde_json::Value> = body
        .options
        .iter()
        .enumerate()
        .map(|(i, text)| serde_json::json!({ "index": i, "text": text }))
        .collect();

    let ends_at: Option<chrono::DateTime<chrono::Utc>> =
        body.ends_at.as_ref().and_then(|s| s.parse().ok());

    let row = sqlx::query(
        "INSERT INTO polls (channel_id, creator_id, question, options, ends_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .bind(&body.question)
    .bind(serde_json::Value::Array(options.clone()))
    .bind(ends_at)
    .fetch_one(state.db.pool())
    .await?;

    let poll_id: Uuid = row.try_get("id")?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "poll_created",
            "poll_id": poll_id,
            "channel_id": channel_id,
            "question": body.question,
            "options": options,
            "ends_at": body.ends_at,
        }),
        &state,
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": poll_id })),
    ))
}

#[derive(serde::Deserialize)]
struct VoteInput {
    option_index: i32,
}

async fn vote_poll(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(poll_id): Path<Uuid>,
    Json(body): Json<VoteInput>,
) -> AppResult<impl IntoResponse> {
    // Fetch poll + server_id for broadcast + expiry check
    let poll_row = sqlx::query(
        "SELECT p.options, p.ends_at, c.server_id
         FROM polls p
         JOIN channels c ON c.id = p.channel_id
         WHERE p.id = $1",
    )
    .bind(poll_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Poll not found".into()))?;

    if let Ok(Some(ends_at)) =
        poll_row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("ends_at")
    {
        if ends_at < chrono::Utc::now() {
            return Err(AppError::BadRequest("Poll has ended".into()));
        }
    }

    let options: serde_json::Value = poll_row.try_get("options")?;
    let option_count = options.as_array().map(|a| a.len()).unwrap_or(0) as i32;
    if body.option_index < 0 || body.option_index >= option_count {
        return Err(AppError::BadRequest("Invalid option_index".into()));
    }

    let result =
        sqlx::query("INSERT INTO poll_votes (poll_id, user_id, option_index) VALUES ($1, $2, $3)")
            .bind(poll_id)
            .bind(auth.user_id)
            .bind(body.option_index)
            .execute(state.db.pool())
            .await;

    if let Err(sqlx::Error::Database(ref e)) = result {
        if e.code().as_deref() == Some("23505") {
            return Err(AppError::Conflict("Already voted on this poll".into()));
        }
    }
    result?;

    // Current vote counts per option
    let vote_rows = sqlx::query(
        "SELECT option_index, COUNT(*) AS cnt
         FROM poll_votes WHERE poll_id = $1
         GROUP BY option_index",
    )
    .bind(poll_id)
    .fetch_all(state.db.pool())
    .await?;

    let vote_counts: serde_json::Map<String, serde_json::Value> = vote_rows
        .iter()
        .filter_map(|r| {
            let idx: i32 = r.try_get("option_index").ok()?;
            let cnt: i64 = r.try_get("cnt").ok()?;
            Some((idx.to_string(), serde_json::json!(cnt)))
        })
        .collect();

    let server_id: Uuid = poll_row.try_get("server_id")?;
    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "poll_vote",
            "poll_id": poll_id,
            "vote_counts": vote_counts,
        }),
        &state,
    )
    .await;

    Ok(Json(serde_json::json!({ "vote_counts": vote_counts })))
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

/// Verify the calling user is a member of `server_id`.
async fn require_member(server_id: Uuid, user_id: Uuid, state: &AppState) -> AppResult<()> {
    let is_member =
        sqlx::query("SELECT 1 FROM server_memberships WHERE server_id = $1 AND user_id = $2")
            .bind(server_id)
            .bind(user_id)
            .fetch_optional(state.db.pool())
            .await?
            .is_some();

    if !is_member {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Verify the calling user is a member of the server that owns `channel_id`.
/// Returns the server_id.
async fn require_channel_member(
    channel_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<Uuid> {
    let row = sqlx::query(
        "SELECT c.server_id
         FROM channels c
         JOIN server_memberships sm ON sm.server_id = c.server_id AND sm.user_id = $2
         WHERE c.id = $1",
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(AppError::Forbidden)?;

    Ok(row.try_get("server_id")?)
}

/// Broadcast a CanvasUpdate event to all connected members of a server.
async fn broadcast_canvas(server_id: Uuid, payload: serde_json::Value, state: &AppState) {
    let rows = sqlx::query("SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT 500")
        .bind(server_id)
        .fetch_all(state.db.pool())
        .await
        .unwrap_or_default();

    let member_ids: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
        .collect();

    state
        .hub
        .broadcast(&member_ids, WsOutbound::CanvasUpdate { payload });
}
