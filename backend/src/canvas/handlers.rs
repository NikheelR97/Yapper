use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::Row;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

use super::{service, types::*};

// ═════════════════════════════════════════════════════════════════════════════
// MUSIC
// ═════════════════════════════════════════════════════════════════════════════

/// PATCH /canvas/servers/:server_id/music — set now-playing directly
pub async fn patch_music(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<MusicInput>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::require_member(server_id, auth.user_id, &state).await?;
    service::set_music(server_id, auth.user_id, &body, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /canvas/servers/:server_id/music/queue — add track to queue
pub async fn enqueue_track(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<EnqueueTrackReq>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::require_admin_or_dj(server_id, auth.user_id, &state).await?;
    let (id, position) = service::enqueue_track(server_id, auth.user_id, &body, &state).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id, "position": position })),
    ))
}

/// DELETE /canvas/servers/:server_id/music/queue/:track_id — remove from queue
pub async fn remove_from_queue(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((server_id, track_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    service::require_member(server_id, auth.user_id, &state).await?;
    service::remove_from_queue(server_id, track_id, auth.user_id, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /canvas/servers/:server_id/music/queue/reorder — reorder queue
pub async fn reorder_queue(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<ReorderQueueReq>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::require_admin_or_dj(server_id, auth.user_id, &state).await?;
    service::reorder_queue(server_id, &body.track_ids, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /canvas/servers/:server_id/music/skip-vote — vote to skip
pub async fn skip_vote(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    service::require_member(server_id, auth.user_id, &state).await?;
    let result = service::skip_vote(server_id, auth.user_id, &state).await?;
    Ok(Json(result))
}

/// POST /canvas/servers/:server_id/music/advance — advance to next track
pub async fn advance_track(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<AdvanceTrackReq>,
) -> AppResult<impl IntoResponse> {
    service::require_member(server_id, auth.user_id, &state).await?;
    if body.force {
        service::require_admin_or_dj(server_id, auth.user_id, &state).await?;
    }
    let np = service::advance_track(server_id, auth.user_id, body.force, &state).await?;
    let remaining: i64 = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM music_queue
         WHERE server_id = $1 AND started_at IS NULL AND finished_at IS NULL",
    )
    .bind(server_id)
    .fetch_one(state.db.pool())
    .await?
    .try_get("cnt")?;
    Ok(Json(serde_json::json!({
        "now_playing": np,
        "queue_remaining": remaining,
    })))
}

/// GET /canvas/servers/:server_id/music/history — track history
pub async fn get_music_history(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    service::require_member(server_id, auth.user_id, &state).await?;
    let tracks = service::get_music_history(server_id, &state).await?;
    Ok(Json(serde_json::json!({ "tracks": tracks })))
}

/// PATCH /canvas/servers/:server_id/music/settings — update music settings
pub async fn update_music_settings(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpdateMusicSettingsReq>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::require_server_admin(server_id, auth.user_id, &state).await?;
    service::update_music_settings(server_id, &body, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /canvas/servers/:server_id/music/dj/:user_id — grant DJ
pub async fn grant_dj(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((server_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    service::require_server_admin(server_id, auth.user_id, &state).await?;
    service::grant_dj(server_id, target_user_id, auth.user_id, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /canvas/servers/:server_id/music/dj/:user_id — revoke DJ
pub async fn revoke_dj(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((server_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    service::require_server_admin(server_id, auth.user_id, &state).await?;
    service::revoke_dj(server_id, target_user_id, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ═════════════════════════════════════════════════════════════════════════════
// POLLS
// ═════════════════════════════════════════════════════════════════════════════

/// POST /canvas/channels/:channel_id/polls — create poll (extended)
pub async fn create_poll(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreatePollReq>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let server_id = service::require_channel_member(channel_id, auth.user_id, &state).await?;
    let poll_id = service::create_poll(channel_id, server_id, auth.user_id, &body, &state).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": poll_id,
            "poll_type": body.poll_type,
            "anonymous": body.anonymous,
        })),
    ))
}

/// POST /canvas/polls/:poll_id/vote — vote on a poll
pub async fn vote_poll(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(poll_id): Path<Uuid>,
    Json(body): Json<VoteReq>,
) -> AppResult<impl IntoResponse> {
    let (result, _server_id) =
        service::vote_poll(poll_id, auth.user_id, body.option_index, &state).await?;
    Ok(Json(result))
}

/// POST /canvas/polls/:poll_id/close — close a poll (admin)
pub async fn close_poll(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(poll_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    service::close_poll(poll_id, auth.user_id, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /canvas/polls/:poll_id/results — poll results
pub async fn get_poll_results(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(poll_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let results = service::get_poll_results(poll_id, auth.user_id, &state).await?;
    Ok(Json(results))
}

// ═════════════════════════════════════════════════════════════════════════════
// CLIPS
// ═════════════════════════════════════════════════════════════════════════════

/// PUT /canvas/clips/:clip_id/reactions — add reaction
pub async fn add_clip_reaction(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(clip_id): Path<Uuid>,
    Json(body): Json<AddReactionReq>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::add_clip_reaction(clip_id, auth.user_id, &body.emoji, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /canvas/clips/:clip_id/reactions?emoji=... — remove reaction
pub async fn remove_clip_reaction(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(clip_id): Path<Uuid>,
    Query(query): Query<RemoveReactionQuery>,
) -> AppResult<impl IntoResponse> {
    query.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::remove_clip_reaction(clip_id, auth.user_id, &query.emoji, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /canvas/servers/:server_id/pinned-clips — pin a clip (admin)
pub async fn pin_clip(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<PinClipReq>,
) -> AppResult<impl IntoResponse> {
    service::require_server_admin(server_id, auth.user_id, &state).await?;
    service::pin_clip(server_id, body.clip_id, auth.user_id, &state).await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /canvas/servers/:server_id/pinned-clips/:clip_id — unpin
pub async fn unpin_clip(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((server_id, clip_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    service::require_server_admin(server_id, auth.user_id, &state).await?;
    service::unpin_clip(server_id, clip_id, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ═════════════════════════════════════════════════════════════════════════════
// EVENTS
// ═════════════════════════════════════════════════════════════════════════════

/// POST /canvas/servers/:server_id/events — create event (admin)
pub async fn create_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateEventReq>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::require_server_admin(server_id, auth.user_id, &state).await?;
    let event = service::create_event(server_id, auth.user_id, &body, &state).await?;
    Ok((StatusCode::CREATED, Json(event)))
}

/// PATCH /canvas/events/:event_id — update event (admin)
pub async fn update_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Json(body): Json<UpdateEventReq>,
) -> AppResult<impl IntoResponse> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let event = service::update_event(event_id, auth.user_id, &body, &state).await?;
    Ok(Json(event))
}

/// DELETE /canvas/events/:event_id — delete event (admin)
pub async fn delete_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    service::delete_event(event_id, auth.user_id, &state).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ═════════════════════════════════════════════════════════════════════════════
// HYDRATION + LEGACY
// ═════════════════════════════════════════════════════════════════════════════

/// GET /canvas/servers/:server_id/state — full canvas state snapshot
pub async fn get_canvas_state(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(query): Query<CanvasStateQuery>,
) -> AppResult<impl IntoResponse> {
    service::require_member(server_id, auth.user_id, &state).await?;
    let result =
        service::get_canvas_state(server_id, query.channel_id, auth.user_id, &state).await?;
    Ok((
        [("cache-control", "private, max-age=5")],
        Json(result),
    ))
}

/// GET /canvas/servers/:server_id — legacy canvas state (music + polls).
/// Kept for backward compatibility.
pub async fn get_canvas_legacy(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    service::require_member(server_id, auth.user_id, &state).await?;

    let music = sqlx::query(
        "SELECT artist, title, source_url, album_art_url, set_by_user_id,
                duration_secs, started_at, updated_at
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
            "duration_secs": r.try_get::<Option<i32>, _>("duration_secs").ok().flatten(),
            "started_at":    r.try_get::<chrono::DateTime<chrono::Utc>, _>("started_at")
                               .ok().map(|t| t.to_rfc3339()),
            "updated_at":    r.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                               .ok().map(|t| t.to_rfc3339()),
        })
    });

    let poll_rows = sqlx::query(
        "SELECT p.id, p.channel_id, p.question, p.options, p.ends_at, p.created_at,
                p.poll_type, p.anonymous, p.status,
                (SELECT jsonb_object_agg(v.option_index::text, v.cnt)
                 FROM (SELECT option_index, COUNT(*) AS cnt
                       FROM poll_votes WHERE poll_id = p.id
                       GROUP BY option_index) v) AS vote_counts
         FROM polls p
         JOIN channels c ON c.id = p.channel_id
         WHERE c.server_id = $1
           AND p.status = 'active'
           AND (p.ends_at IS NULL OR p.ends_at > NOW())
         ORDER BY p.created_at DESC
         LIMIT 10",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    let polls: Vec<serde_json::Value> = poll_rows
        .iter()
        .filter_map(|r| {
            Some(serde_json::json!({
                "id":          r.try_get::<Uuid, _>("id").ok()?,
                "channel_id":  r.try_get::<Uuid, _>("channel_id").ok()?,
                "poll_type":   r.try_get::<String, _>("poll_type").ok()?,
                "question":    r.try_get::<String, _>("question").ok()?,
                "options":     r.try_get::<serde_json::Value, _>("options").ok()?,
                "vote_counts": r.try_get::<Option<serde_json::Value>, _>("vote_counts")
                                 .ok().flatten().unwrap_or(serde_json::json!({})),
                "anonymous":   r.try_get::<bool, _>("anonymous").ok()?,
                "status":      r.try_get::<String, _>("status").ok()?,
                "ends_at":     r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("ends_at")
                                 .ok().flatten().map(|t| t.to_rfc3339()),
                "created_at":  r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                 .ok().map(|t| t.to_rfc3339()),
            }))
        })
        .collect();

    // Clips (same as before but with 30-day expiry filter)
    let clip_rows = sqlx::query(
        "SELECT m.id, m.sender_id, m.ciphertext, m.created_at, c.id AS channel_id
         FROM messages m
         JOIN channels c ON c.id = m.channel_id
         WHERE c.server_id = $1
           AND m.message_type = 'clip'
           AND m.deleted_at IS NULL
           AND m.ciphertext IS NOT NULL
           AND m.created_at > NOW() - INTERVAL '30 days'
         ORDER BY m.created_at DESC
         LIMIT 20",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    let clips: Vec<serde_json::Value> = clip_rows
        .iter()
        .filter_map(|r| {
            let ciphertext = r
                .try_get::<Vec<u8>, _>("ciphertext")
                .ok()
                .map(|b| BASE64.encode(&b));
            Some(serde_json::json!({
                "id":         r.try_get::<Uuid, _>("id").ok()?,
                "channel_id": r.try_get::<Uuid, _>("channel_id").ok()?,
                "sender_id":  r.try_get::<Uuid, _>("sender_id").ok()?,
                "ciphertext": ciphertext,
                "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                .ok().map(|t| t.to_rfc3339()),
            }))
        })
        .collect();

    Ok(Json(serde_json::json!({
        "music": music,
        "polls": polls,
        "clips": clips,
    })))
}
