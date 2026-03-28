use chrono::{DateTime, Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    hub::WsOutbound,
    AppState,
};

use super::types::*;

// ─── Auth helpers ────────────────────────────────────────────────────────────

/// Verify the calling user is a member of `server_id`.
pub async fn require_member(server_id: Uuid, user_id: Uuid, state: &AppState) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

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
pub async fn require_channel_member(
    channel_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<Uuid> {
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

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

/// Verify the calling user is admin/owner of the server, OR has DJ permission.
pub async fn require_admin_or_dj(
    server_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let row = sqlx::query(
        "SELECT 1 FROM server_memberships sm
         WHERE sm.user_id = $1 AND sm.server_id = $2
           AND (sm.role IN ('owner', 'admin')
                OR EXISTS (SELECT 1 FROM canvas_dj_users dj
                           WHERE dj.server_id = $2 AND dj.user_id = $1))",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    if row.is_none() {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Verify the calling user is admin/owner of the server.
pub async fn require_server_admin(
    server_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let row = sqlx::query(
        "SELECT 1 FROM server_memberships
         WHERE user_id = $1 AND server_id = $2 AND role IN ('owner', 'admin')",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    if row.is_none() {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Broadcast a CanvasUpdate event to all connected members of a server.
pub async fn broadcast_canvas(server_id: Uuid, payload: serde_json::Value, state: &AppState) {
    let member_ids = get_server_member_ids(server_id, state).await;
    state
        .hub
        .broadcast(&member_ids, WsOutbound::CanvasUpdate { payload });
}

/// Fetch up to 500 member user IDs for a server.
pub async fn get_server_member_ids(server_id: Uuid, state: &AppState) -> Vec<Uuid> {
    sqlx::query("SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT 500")
        .bind(server_id)
        .fetch_all(state.db.pool())
        .await
        .unwrap_or_default()
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
        .collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// MUSIC
// ═════════════════════════════════════════════════════════════════════════════

/// Set music state directly (for bots or manual "now playing").
pub async fn set_music(
    server_id: Uuid,
    user_id: Uuid,
    body: &MusicInput,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    sqlx::query(
        "INSERT INTO canvas_music_state
            (server_id, artist, title, source_url, album_art_url,
             set_by_user_id, duration_secs, started_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
         ON CONFLICT (server_id) DO UPDATE SET
            artist          = EXCLUDED.artist,
            title           = EXCLUDED.title,
            source_url      = EXCLUDED.source_url,
            album_art_url   = EXCLUDED.album_art_url,
            set_by_user_id  = EXCLUDED.set_by_user_id,
            duration_secs   = EXCLUDED.duration_secs,
            started_at      = NOW(),
            current_track_id = NULL,
            updated_at      = NOW()",
    )
    .bind(server_id)
    .bind(&body.artist)
    .bind(&body.title)
    .bind(&body.source_url)
    .bind(&body.album_art_url)
    .bind(user_id)
    .bind(body.duration_secs)
    .execute(state.db.pool())
    .await?;

    // Clear any skip votes from the old track
    sqlx::query("DELETE FROM music_skip_votes WHERE server_id = $1")
        .bind(server_id)
        .execute(state.db.pool())
        .await?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "music_track_started",
            "server_id": server_id,
            "track_id": serde_json::Value::Null,
            "artist": body.artist,
            "title": body.title,
            "duration_secs": body.duration_secs,
            "source_url": body.source_url,
            "album_art_url": body.album_art_url,
            "started_at": Utc::now().to_rfc3339(),
            "set_by": user_id,
        }),
        state,
    )
    .await;

    Ok(())
}

/// Add a track to the queue.
pub async fn enqueue_track(
    server_id: Uuid,
    user_id: Uuid,
    body: &EnqueueTrackReq,
    state: &AppState,
) -> AppResult<(Uuid, i32)> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    // Check queue cap
    let count: i64 = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM music_queue
         WHERE server_id = $1 AND started_at IS NULL AND finished_at IS NULL",
    )
    .bind(server_id)
    .fetch_one(state.db.pool())
    .await?
    .try_get("cnt")?;

    if count >= MAX_MUSIC_QUEUE_SIZE {
        return Err(AppError::Conflict("Music queue is full (max 50)".into()));
    }

    let next_pos: i32 = sqlx::query(
        "SELECT COALESCE(MAX(position), -1) + 1 AS next_pos FROM music_queue
         WHERE server_id = $1 AND started_at IS NULL AND finished_at IS NULL",
    )
    .bind(server_id)
    .fetch_one(state.db.pool())
    .await?
    .try_get("next_pos")?;

    let row = sqlx::query(
        "INSERT INTO music_queue
            (server_id, added_by, artist, title, duration_secs,
             source_url, album_art_url, position)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id",
    )
    .bind(server_id)
    .bind(user_id)
    .bind(&body.artist)
    .bind(&body.title)
    .bind(body.duration_secs)
    .bind(&body.source_url)
    .bind(&body.album_art_url)
    .bind(next_pos)
    .fetch_one(state.db.pool())
    .await?;

    let track_id: Uuid = row.try_get("id")?;

    // If nothing is currently playing, auto-start this track
    let has_now_playing = sqlx::query("SELECT 1 FROM canvas_music_state WHERE server_id = $1")
        .bind(server_id)
        .fetch_optional(state.db.pool())
        .await?
        .is_some();

    if !has_now_playing {
        advance_track_inner(server_id, state).await?;
    } else {
        broadcast_queue_updated(server_id, state).await;
    }

    Ok((track_id, next_pos))
}

/// Remove a track from the queue.
pub async fn remove_from_queue(
    server_id: Uuid,
    track_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(track_id != Uuid::nil());

    // Verify track exists, belongs to server, is queued (not playing/finished)
    let row = sqlx::query(
        "SELECT added_by, position FROM music_queue
         WHERE id = $1 AND server_id = $2
           AND started_at IS NULL AND finished_at IS NULL",
    )
    .bind(track_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Track not found in queue".into()))?;

    let added_by: Uuid = row.try_get("added_by")?;
    let position: i32 = row.try_get("position")?;

    // Check permission: admin/DJ or the user who added it
    if added_by != user_id {
        require_admin_or_dj(server_id, user_id, state).await?;
    }

    sqlx::query("DELETE FROM music_queue WHERE id = $1")
        .bind(track_id)
        .execute(state.db.pool())
        .await?;

    // Re-compact positions
    sqlx::query(
        "UPDATE music_queue SET position = position - 1
         WHERE server_id = $1 AND position > $2
           AND started_at IS NULL AND finished_at IS NULL",
    )
    .bind(server_id)
    .bind(position)
    .execute(state.db.pool())
    .await?;

    broadcast_queue_updated(server_id, state).await;
    Ok(())
}

/// Reorder the queue. `track_ids` must be a permutation of all active queue entries.
pub async fn reorder_queue(server_id: Uuid, track_ids: &[Uuid], state: &AppState) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());

    let active_rows = sqlx::query(
        "SELECT id FROM music_queue
         WHERE server_id = $1 AND started_at IS NULL AND finished_at IS NULL
         ORDER BY position",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    let active_ids: Vec<Uuid> = active_rows
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("id").ok())
        .collect();

    // Verify it's a permutation
    if track_ids.len() != active_ids.len() {
        return Err(AppError::BadRequest(
            "track_ids must include all active queue entries".into(),
        ));
    }

    let mut sorted_input = track_ids.to_vec();
    sorted_input.sort();
    let mut sorted_active = active_ids.clone();
    sorted_active.sort();
    if sorted_input != sorted_active {
        return Err(AppError::BadRequest(
            "track_ids must be a permutation of the active queue".into(),
        ));
    }

    for (i, tid) in track_ids
        .iter()
        .enumerate()
        .take(MAX_MUSIC_QUEUE_SIZE as usize)
    {
        sqlx::query("UPDATE music_queue SET position = $1 WHERE id = $2")
            .bind(i as i32)
            .bind(tid)
            .execute(state.db.pool())
            .await?;
    }

    broadcast_queue_updated(server_id, state).await;
    Ok(())
}

/// Vote to skip the current track.
pub async fn skip_vote(
    server_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    // Verify something is playing
    let has_music = sqlx::query("SELECT 1 FROM canvas_music_state WHERE server_id = $1")
        .bind(server_id)
        .fetch_optional(state.db.pool())
        .await?;

    if has_music.is_none() {
        return Err(AppError::NotFound("No track currently playing".into()));
    }

    // Cast vote (idempotent)
    let result = sqlx::query(
        "INSERT INTO music_skip_votes (server_id, user_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(server_id)
    .bind(user_id)
    .execute(state.db.pool())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::Conflict("Already voted to skip".into()));
    }

    // Count votes
    let skip_count: i64 =
        sqlx::query("SELECT COUNT(*) AS cnt FROM music_skip_votes WHERE server_id = $1")
            .bind(server_id)
            .fetch_one(state.db.pool())
            .await?
            .try_get("cnt")?;

    // Get online member count
    let member_ids = get_server_member_ids(server_id, state).await;
    let online_count = state.hub.count_online(&member_ids).max(1);

    // Get threshold
    let threshold: i16 =
        sqlx::query("SELECT skip_threshold_pct FROM canvas_music_settings WHERE server_id = $1")
            .bind(server_id)
            .fetch_optional(state.db.pool())
            .await?
            .and_then(|r| r.try_get("skip_threshold_pct").ok())
            .unwrap_or(DEFAULT_SKIP_THRESHOLD_PCT);

    let threshold_met = skip_count * 100 >= (threshold as i64) * (online_count as i64);

    if threshold_met {
        let now_playing = advance_track_inner(server_id, state).await?;
        broadcast_canvas(
            server_id,
            serde_json::json!({
                "type": "music_track_skipped",
                "server_id": server_id,
                "now_playing": now_playing,
            }),
            state,
        )
        .await;

        return Ok(serde_json::json!({
            "skip_votes": skip_count,
            "online_members": online_count,
            "threshold_pct": threshold,
            "skipped": true,
            "now_playing": now_playing,
        }));
    }

    // Broadcast vote update
    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "music_skip_vote_update",
            "server_id": server_id,
            "skip_votes": skip_count,
            "online_members": online_count,
            "threshold_pct": threshold,
        }),
        state,
    )
    .await;

    Ok(serde_json::json!({
        "skip_votes": skip_count,
        "online_members": online_count,
        "threshold_pct": threshold,
        "skipped": false,
    }))
}

/// Advance to the next track. If `force` is false, checks that the current
/// track has elapsed before advancing.
pub async fn advance_track(
    server_id: Uuid,
    user_id: Uuid,
    force: bool,
    state: &AppState,
) -> AppResult<Option<serde_json::Value>> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    if !force {
        // Check if current track has elapsed
        let row = sqlx::query(
            "SELECT started_at, duration_secs FROM canvas_music_state WHERE server_id = $1",
        )
        .bind(server_id)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("No track currently playing".into()))?;

        let started_at: DateTime<Utc> = row.try_get("started_at")?;
        let duration: Option<i32> = row.try_get("duration_secs").ok().flatten();

        if let Some(dur) = duration {
            let elapsed = Utc::now() - started_at;
            if elapsed < Duration::seconds(dur as i64) {
                return Err(AppError::Conflict("Track has not finished yet".into()));
            }
        } else {
            // No duration = indefinite; only admin can advance
            require_admin_or_dj(server_id, user_id, state).await?;
        }
    }

    let now_playing = advance_track_inner(server_id, state).await?;

    let event_type = if now_playing.is_some() {
        "music_track_started"
    } else {
        "music_stopped"
    };

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": event_type,
            "server_id": server_id,
            "now_playing": now_playing,
        }),
        state,
    )
    .await;

    Ok(now_playing)
}

/// Internal: advance to the next queue item. Returns the new now-playing, or
/// None if queue is empty.
async fn advance_track_inner(
    server_id: Uuid,
    state: &AppState,
) -> AppResult<Option<serde_json::Value>> {
    // Mark current queue entry as finished
    sqlx::query(
        "UPDATE music_queue SET finished_at = NOW()
         WHERE server_id = $1 AND started_at IS NOT NULL AND finished_at IS NULL",
    )
    .bind(server_id)
    .execute(state.db.pool())
    .await?;

    // Pop next from queue
    let next = sqlx::query(
        "UPDATE music_queue SET started_at = NOW()
         WHERE id = (
             SELECT id FROM music_queue
             WHERE server_id = $1 AND started_at IS NULL AND finished_at IS NULL
             ORDER BY position LIMIT 1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, artist, title, duration_secs, source_url, album_art_url, added_by",
    )
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    // Clear skip votes
    sqlx::query("DELETE FROM music_skip_votes WHERE server_id = $1")
        .bind(server_id)
        .execute(state.db.pool())
        .await?;

    if let Some(row) = next {
        let track_id: Uuid = row.try_get("id")?;
        let artist: String = row.try_get("artist")?;
        let title: String = row.try_get("title")?;
        let duration_secs: i32 = row.try_get("duration_secs")?;
        let source_url: Option<String> = row.try_get("source_url").ok().flatten();
        let album_art_url: Option<String> = row.try_get("album_art_url").ok().flatten();
        let added_by: Uuid = row.try_get("added_by")?;
        let now = Utc::now();

        // Update canvas_music_state
        sqlx::query(
            "INSERT INTO canvas_music_state
                (server_id, artist, title, source_url, album_art_url,
                 set_by_user_id, duration_secs, started_at, current_track_id, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $8)
             ON CONFLICT (server_id) DO UPDATE SET
                artist          = EXCLUDED.artist,
                title           = EXCLUDED.title,
                source_url      = EXCLUDED.source_url,
                album_art_url   = EXCLUDED.album_art_url,
                set_by_user_id  = EXCLUDED.set_by_user_id,
                duration_secs   = EXCLUDED.duration_secs,
                started_at      = EXCLUDED.started_at,
                current_track_id = EXCLUDED.current_track_id,
                updated_at      = EXCLUDED.updated_at",
        )
        .bind(server_id)
        .bind(&artist)
        .bind(&title)
        .bind(&source_url)
        .bind(&album_art_url)
        .bind(added_by)
        .bind(duration_secs)
        .bind(now)
        .bind(track_id)
        .execute(state.db.pool())
        .await?;

        // Trim history beyond 20
        sqlx::query(
            "DELETE FROM music_queue
             WHERE server_id = $1 AND finished_at IS NOT NULL
               AND id NOT IN (
                   SELECT id FROM music_queue
                   WHERE server_id = $1 AND finished_at IS NOT NULL
                   ORDER BY finished_at DESC LIMIT $2
               )",
        )
        .bind(server_id)
        .bind(MAX_MUSIC_HISTORY_SIZE)
        .execute(state.db.pool())
        .await?;

        let np = serde_json::json!({
            "track_id": track_id,
            "artist": artist,
            "title": title,
            "duration_secs": duration_secs,
            "source_url": source_url,
            "album_art_url": album_art_url,
            "started_at": now.to_rfc3339(),
            "set_by": added_by,
        });

        return Ok(Some(np));
    }

    // Queue empty — clear now-playing
    sqlx::query("DELETE FROM canvas_music_state WHERE server_id = $1")
        .bind(server_id)
        .execute(state.db.pool())
        .await?;

    Ok(None)
}

/// Get music history (last 20 finished tracks).
pub async fn get_music_history(
    server_id: Uuid,
    state: &AppState,
) -> AppResult<Vec<HistoryTrackResp>> {
    debug_assert!(server_id != Uuid::nil());

    let rows = sqlx::query(
        "SELECT id, artist, title, duration_secs, source_url, album_art_url,
                added_by, started_at, finished_at
         FROM music_queue
         WHERE server_id = $1 AND finished_at IS NOT NULL
         ORDER BY finished_at DESC
         LIMIT $2",
    )
    .bind(server_id)
    .bind(MAX_MUSIC_HISTORY_SIZE)
    .fetch_all(state.db.pool())
    .await?;

    rows.iter()
        .map(|r| -> Result<HistoryTrackResp, sqlx::Error> {
            Ok(HistoryTrackResp {
                id: r.try_get("id")?,
                artist: r.try_get("artist")?,
                title: r.try_get("title")?,
                duration_secs: r.try_get("duration_secs")?,
                source_url: r.try_get("source_url").ok().flatten(),
                album_art_url: r.try_get("album_art_url").ok().flatten(),
                added_by: r.try_get("added_by")?,
                started_at: r.try_get("started_at").ok().flatten(),
                finished_at: r.try_get("finished_at").ok().flatten(),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)
}

/// Update music settings for a server.
pub async fn update_music_settings(
    server_id: Uuid,
    body: &UpdateMusicSettingsReq,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());

    sqlx::query(
        "INSERT INTO canvas_music_settings (server_id, skip_threshold_pct, queue_enabled)
         VALUES ($1, COALESCE($2, 50), COALESCE($3, TRUE))
         ON CONFLICT (server_id) DO UPDATE SET
            skip_threshold_pct = COALESCE($2, canvas_music_settings.skip_threshold_pct),
            queue_enabled      = COALESCE($3, canvas_music_settings.queue_enabled)",
    )
    .bind(server_id)
    .bind(body.skip_threshold_pct)
    .bind(body.queue_enabled)
    .execute(state.db.pool())
    .await?;

    Ok(())
}

/// Grant DJ role to a user.
pub async fn grant_dj(
    server_id: Uuid,
    target_user_id: Uuid,
    granted_by: Uuid,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(target_user_id != Uuid::nil());

    // Verify target is a server member
    require_member(server_id, target_user_id, state).await?;

    // Check cap
    let count: i64 =
        sqlx::query("SELECT COUNT(*) AS cnt FROM canvas_dj_users WHERE server_id = $1")
            .bind(server_id)
            .fetch_one(state.db.pool())
            .await?
            .try_get("cnt")?;

    if count >= MAX_DJ_USERS_PER_SERVER {
        return Err(AppError::Conflict("DJ user limit reached (max 20)".into()));
    }

    sqlx::query(
        "INSERT INTO canvas_dj_users (server_id, user_id, granted_by)
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(server_id)
    .bind(target_user_id)
    .bind(granted_by)
    .execute(state.db.pool())
    .await?;

    Ok(())
}

/// Revoke DJ role from a user.
pub async fn revoke_dj(server_id: Uuid, target_user_id: Uuid, state: &AppState) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(target_user_id != Uuid::nil());

    sqlx::query("DELETE FROM canvas_dj_users WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(target_user_id)
        .execute(state.db.pool())
        .await?;

    Ok(())
}

/// Broadcast the full queue to all server members.
async fn broadcast_queue_updated(server_id: Uuid, state: &AppState) {
    let queue = fetch_active_queue(server_id, state).await;
    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "music_queue_updated",
            "server_id": server_id,
            "queue": queue,
        }),
        state,
    )
    .await;
}

/// Fetch the active (unstarted, unfinished) queue for a server.
async fn fetch_active_queue(server_id: Uuid, state: &AppState) -> Vec<serde_json::Value> {
    let rows = sqlx::query(
        "SELECT id, artist, title, duration_secs, source_url, album_art_url,
                added_by, position
         FROM music_queue
         WHERE server_id = $1 AND started_at IS NULL AND finished_at IS NULL
         ORDER BY position
         LIMIT $2",
    )
    .bind(server_id)
    .bind(MAX_MUSIC_QUEUE_SIZE)
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();

    rows.iter()
        .filter_map(|r| {
            Some(serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok()?,
                "artist": r.try_get::<String, _>("artist").ok()?,
                "title": r.try_get::<String, _>("title").ok()?,
                "duration_secs": r.try_get::<i32, _>("duration_secs").ok()?,
                "source_url": r.try_get::<Option<String>, _>("source_url").ok().flatten(),
                "album_art_url": r.try_get::<Option<String>, _>("album_art_url").ok().flatten(),
                "added_by": r.try_get::<Uuid, _>("added_by").ok()?,
                "position": r.try_get::<i32, _>("position").ok()?,
            }))
        })
        .collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// POLLS
// ═════════════════════════════════════════════════════════════════════════════

/// Create a poll with type/anonymous support.
pub async fn create_poll(
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    body: &CreatePollReq,
    state: &AppState,
) -> AppResult<Uuid> {
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    // Validate poll_type
    let poll_type = body.poll_type.as_str();
    if !matches!(poll_type, "binary" | "multiple_choice" | "emoji_reaction") {
        return Err(AppError::BadRequest("Invalid poll_type".into()));
    }

    // Build options
    let default_binary = vec!["Yes".to_string(), "No".to_string()];
    let options: Vec<serde_json::Value> = match poll_type {
        "binary" => {
            let labels = body.options.as_deref().unwrap_or(&default_binary);
            if labels.len() != 2 {
                return Err(AppError::BadRequest(
                    "Binary poll must have exactly 2 options".into(),
                ));
            }
            labels
                .iter()
                .enumerate()
                .map(|(i, t)| serde_json::json!({"index": i, "text": t}))
                .collect()
        }
        _ => {
            let labels = body
                .options
                .as_ref()
                .ok_or_else(|| AppError::BadRequest("options required".into()))?;
            if labels.len() < 2 || labels.len() > MAX_POLL_OPTIONS {
                return Err(AppError::BadRequest(format!(
                    "Poll must have 2–{MAX_POLL_OPTIONS} options"
                )));
            }
            let max_len = if poll_type == "emoji_reaction" {
                8
            } else {
                MAX_POLL_OPTION_LEN
            };
            for opt in labels {
                if opt.is_empty() || opt.len() > max_len {
                    return Err(AppError::BadRequest(format!(
                        "Option text must be 1–{max_len} chars"
                    )));
                }
            }
            labels
                .iter()
                .enumerate()
                .map(|(i, t)| serde_json::json!({"index": i, "text": t}))
                .collect()
        }
    };

    // Check active poll cap
    let active_count: i64 = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM polls
         WHERE channel_id = $1 AND status = 'active'
           AND (ends_at IS NULL OR ends_at > NOW())",
    )
    .bind(channel_id)
    .fetch_one(state.db.pool())
    .await?
    .try_get("cnt")?;

    if active_count >= MAX_ACTIVE_POLLS_PER_CHANNEL {
        return Err(AppError::Conflict(format!(
            "Max {MAX_ACTIVE_POLLS_PER_CHANNEL} active polls per channel"
        )));
    }

    // Parse ends_at
    let ends_at: Option<DateTime<Utc>> = body.ends_at.as_ref().and_then(|s| {
        s.parse()
            .inspect_err(|e| {
                tracing::warn!(
                    input = %s,
                    error = %e,
                    "Failed to parse ends_at datetime, ignoring"
                );
            })
            .ok()
    });
    if let Some(ea) = ends_at {
        if ea <= Utc::now() {
            return Err(AppError::BadRequest("ends_at must be in the future".into()));
        }
        let max_end = Utc::now() + Duration::seconds(MAX_POLL_DURATION_SECS);
        if ea > max_end {
            return Err(AppError::BadRequest(
                "ends_at must be within 24 hours".into(),
            ));
        }
    }

    let row = sqlx::query(
        "INSERT INTO polls (channel_id, creator_id, question, options,
                            ends_at, poll_type, anonymous)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(&body.question)
    .bind(serde_json::Value::Array(options.clone()))
    .bind(ends_at)
    .bind(poll_type)
    .bind(body.anonymous)
    .fetch_one(state.db.pool())
    .await?;

    let poll_id: Uuid = row.try_get("id")?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "poll_created",
            "server_id": server_id,
            "poll_id": poll_id,
            "channel_id": channel_id,
            "poll_type": poll_type,
            "question": body.question,
            "options": options,
            "anonymous": body.anonymous,
            "ends_at": body.ends_at,
        }),
        state,
    )
    .await;

    Ok(poll_id)
}

/// Vote on a poll.
pub async fn vote_poll(
    poll_id: Uuid,
    user_id: Uuid,
    option_index: i32,
    state: &AppState,
) -> AppResult<(serde_json::Value, Uuid)> {
    debug_assert!(poll_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let poll_row = sqlx::query(
        "SELECT p.options, p.ends_at, p.status, c.server_id
         FROM polls p
         JOIN channels c ON c.id = p.channel_id
         WHERE p.id = $1",
    )
    .bind(poll_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Poll not found".into()))?;

    let status: String = poll_row.try_get("status")?;
    if status == "closed" {
        return Err(AppError::BadRequest("Poll is closed".into()));
    }

    if let Ok(Some(ends_at)) = poll_row.try_get::<Option<DateTime<Utc>>, _>("ends_at") {
        if ends_at < Utc::now() {
            return Err(AppError::BadRequest("Poll has ended".into()));
        }
    }

    let options: serde_json::Value = poll_row.try_get("options")?;
    let option_count = options.as_array().map(|a| a.len()).unwrap_or(0) as i32;
    if option_index < 0 || option_index >= option_count {
        return Err(AppError::BadRequest("Invalid option_index".into()));
    }
    let server_id: Uuid = poll_row.try_get("server_id")?;
    require_member(server_id, user_id, state).await?;

    let result =
        sqlx::query("INSERT INTO poll_votes (poll_id, user_id, option_index) VALUES ($1, $2, $3)")
            .bind(poll_id)
            .bind(user_id)
            .bind(option_index)
            .execute(state.db.pool())
            .await;

    if let Err(sqlx::Error::Database(ref e)) = result {
        if e.code().as_deref() == Some("23505") {
            return Err(AppError::Conflict("Already voted on this poll".into()));
        }
    }
    result?;

    let vote_counts = fetch_vote_counts(poll_id, state).await?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "poll_vote",
            "poll_id": poll_id,
            "vote_counts": vote_counts,
        }),
        state,
    )
    .await;

    Ok((serde_json::json!({ "vote_counts": vote_counts }), server_id))
}

/// Close a poll (admin only).
pub async fn close_poll(poll_id: Uuid, user_id: Uuid, state: &AppState) -> AppResult<Uuid> {
    debug_assert!(poll_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let poll_row = sqlx::query(
        "SELECT p.status, c.server_id
         FROM polls p JOIN channels c ON c.id = p.channel_id
         WHERE p.id = $1",
    )
    .bind(poll_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Poll not found".into()))?;

    let status: String = poll_row.try_get("status")?;
    if status == "closed" {
        return Err(AppError::Conflict("Poll already closed".into()));
    }

    let server_id: Uuid = poll_row.try_get("server_id")?;
    require_server_admin(server_id, user_id, state).await?;

    sqlx::query(
        "UPDATE polls SET status = 'closed', closed_at = NOW(), closed_by = $1 WHERE id = $2",
    )
    .bind(user_id)
    .bind(poll_id)
    .execute(state.db.pool())
    .await?;

    let vote_counts = fetch_vote_counts(poll_id, state).await?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "poll_closed",
            "server_id": server_id,
            "poll_id": poll_id,
            "closed_by": user_id,
            "final_vote_counts": vote_counts,
        }),
        state,
    )
    .await;

    Ok(server_id)
}

/// Get poll results with optional voter breakdown (admin only, non-anonymous).
pub async fn get_poll_results(
    poll_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    debug_assert!(poll_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let poll_row = sqlx::query(
        "SELECT p.question, p.options, p.poll_type, p.anonymous, p.status,
                p.ends_at, c.server_id
         FROM polls p JOIN channels c ON c.id = p.channel_id
         WHERE p.id = $1",
    )
    .bind(poll_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Poll not found".into()))?;

    let server_id: Uuid = poll_row.try_get("server_id")?;
    require_member(server_id, user_id, state).await?;

    let anonymous: bool = poll_row.try_get("anonymous")?;
    let is_admin = sqlx::query(
        "SELECT 1 FROM server_memberships
         WHERE user_id = $1 AND server_id = $2 AND role IN ('owner', 'admin')",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?
    .is_some();

    let options: serde_json::Value = poll_row.try_get("options")?;
    let vote_counts = fetch_vote_counts(poll_id, state).await?;

    // Build per-option results
    let opt_arr = options.as_array().cloned().unwrap_or_default();
    let mut results = Vec::with_capacity(opt_arr.len());
    let total: i64 = vote_counts
        .as_object()
        .map(|m| m.values().filter_map(|v| v.as_i64()).sum())
        .unwrap_or(0);

    for opt in &opt_arr {
        let idx = opt["index"].as_i64().unwrap_or(0);
        let count = vote_counts
            .get(idx.to_string())
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let voters = if !anonymous && is_admin {
            let voter_rows = sqlx::query(
                "SELECT user_id FROM poll_votes
                 WHERE poll_id = $1 AND option_index = $2
                 LIMIT 100",
            )
            .bind(poll_id)
            .bind(idx as i32)
            .fetch_all(state.db.pool())
            .await
            .unwrap_or_default();

            let ids: Vec<Uuid> = voter_rows
                .iter()
                .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
                .collect();
            Some(serde_json::json!(ids))
        } else {
            None
        };

        results.push(serde_json::json!({
            "index": idx,
            "text": opt["text"],
            "votes": count,
            "voters": voters,
        }));
    }

    Ok(serde_json::json!({
        "poll_id": poll_id,
        "poll_type": poll_row.try_get::<String, _>("poll_type")?,
        "question": poll_row.try_get::<String, _>("question")?,
        "anonymous": anonymous,
        "status": poll_row.try_get::<String, _>("status")?,
        "total_votes": total,
        "options": results,
    }))
}

/// Fetch vote counts for a poll as a JSON object {"0": 5, "1": 3, ...}.
async fn fetch_vote_counts(poll_id: Uuid, state: &AppState) -> AppResult<serde_json::Value> {
    let rows = sqlx::query(
        "SELECT option_index, COUNT(*) AS cnt
         FROM poll_votes WHERE poll_id = $1
         GROUP BY option_index",
    )
    .bind(poll_id)
    .fetch_all(state.db.pool())
    .await?;

    let map: serde_json::Map<String, serde_json::Value> = rows
        .iter()
        .filter_map(|r| {
            let idx: i32 = r.try_get("option_index").ok()?;
            let cnt: i64 = r.try_get("cnt").ok()?;
            Some((idx.to_string(), serde_json::json!(cnt)))
        })
        .collect();

    Ok(serde_json::Value::Object(map))
}

// ═════════════════════════════════════════════════════════════════════════════
// CLIPS
// ═════════════════════════════════════════════════════════════════════════════

/// Add a reaction to a clip.
pub async fn add_clip_reaction(
    clip_id: Uuid,
    user_id: Uuid,
    emoji: &str,
    state: &AppState,
) -> AppResult<Option<Uuid>> {
    debug_assert!(clip_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    // Verify clip exists and get server_id
    let clip_row = sqlx::query(
        "SELECT c.server_id FROM messages m
         JOIN channels c ON c.id = m.channel_id
         WHERE m.id = $1 AND m.message_type = 'clip' AND m.deleted_at IS NULL",
    )
    .bind(clip_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Clip not found".into()))?;

    let server_id: Uuid = clip_row.try_get("server_id")?;
    require_member(server_id, user_id, state).await?;

    // Check reaction cap
    let count: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM clip_reactions WHERE clip_id = $1")
        .bind(clip_id)
        .fetch_one(state.db.pool())
        .await?
        .try_get("cnt")?;

    if count >= MAX_REACTIONS_PER_CLIP {
        return Err(AppError::Conflict("Reaction limit reached".into()));
    }

    let result = sqlx::query(
        "INSERT INTO clip_reactions (clip_id, user_id, emoji)
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(clip_id)
    .bind(user_id)
    .bind(emoji)
    .execute(state.db.pool())
    .await?;

    if result.rows_affected() > 0 {
        let counts = fetch_clip_reaction_counts(clip_id, state).await?;
        broadcast_canvas(
            server_id,
            serde_json::json!({
                "type": "clip_reaction_added",
                "server_id": server_id,
                "clip_id": clip_id,
                "emoji": emoji,
                "user_id": user_id,
                "reaction_counts": counts,
            }),
            state,
        )
        .await;
        return Ok(Some(server_id));
    }

    Ok(None) // Already reacted — idempotent no-op
}

/// Remove a reaction from a clip.
pub async fn remove_clip_reaction(
    clip_id: Uuid,
    user_id: Uuid,
    emoji: &str,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(clip_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    // Get server_id for broadcast
    let clip_row = sqlx::query(
        "SELECT c.server_id FROM messages m
         JOIN channels c ON c.id = m.channel_id
         WHERE m.id = $1 AND m.message_type = 'clip'",
    )
    .bind(clip_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Clip not found".into()))?;

    let server_id: Uuid = clip_row.try_get("server_id")?;
    require_member(server_id, user_id, state).await?;

    let result = sqlx::query(
        "DELETE FROM clip_reactions WHERE clip_id = $1 AND user_id = $2 AND emoji = $3",
    )
    .bind(clip_id)
    .bind(user_id)
    .bind(emoji)
    .execute(state.db.pool())
    .await?;

    if result.rows_affected() > 0 {
        let counts = fetch_clip_reaction_counts(clip_id, state).await?;
        broadcast_canvas(
            server_id,
            serde_json::json!({
                "type": "clip_reaction_removed",
                "server_id": server_id,
                "clip_id": clip_id,
                "emoji": emoji,
                "user_id": user_id,
                "reaction_counts": counts,
            }),
            state,
        )
        .await;
    }

    Ok(())
}

/// Fetch reaction counts for a clip: {"👍": 5, "🔥": 3}
async fn fetch_clip_reaction_counts(
    clip_id: Uuid,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    let rows = sqlx::query(
        "SELECT emoji, COUNT(*) AS cnt FROM clip_reactions
         WHERE clip_id = $1 GROUP BY emoji",
    )
    .bind(clip_id)
    .fetch_all(state.db.pool())
    .await?;

    let map: serde_json::Map<String, serde_json::Value> = rows
        .iter()
        .filter_map(|r| {
            let emoji: String = r.try_get("emoji").ok()?;
            let cnt: i64 = r.try_get("cnt").ok()?;
            Some((emoji, serde_json::json!(cnt)))
        })
        .collect();

    Ok(serde_json::Value::Object(map))
}

/// Pin a clip.
pub async fn pin_clip(
    server_id: Uuid,
    clip_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(clip_id != Uuid::nil());

    // Verify clip belongs to this server
    let exists = sqlx::query(
        "SELECT 1 FROM messages m
         JOIN channels c ON c.id = m.channel_id
         WHERE m.id = $1 AND c.server_id = $2
           AND m.message_type = 'clip' AND m.deleted_at IS NULL",
    )
    .bind(clip_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Clip not found in this server".into()));
    }

    // Check pin cap
    let count: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM pinned_clips WHERE server_id = $1")
        .bind(server_id)
        .fetch_one(state.db.pool())
        .await?
        .try_get("cnt")?;

    if count >= MAX_PINNED_CLIPS {
        return Err(AppError::Conflict(format!(
            "Max {MAX_PINNED_CLIPS} pinned clips"
        )));
    }

    let result = sqlx::query(
        "INSERT INTO pinned_clips (server_id, clip_id, pinned_by)
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(server_id)
    .bind(clip_id)
    .bind(user_id)
    .execute(state.db.pool())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::Conflict("Clip already pinned".into()));
    }

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "clip_pinned",
            "server_id": server_id,
            "clip_id": clip_id,
            "pinned_by": user_id,
        }),
        state,
    )
    .await;

    Ok(())
}

/// Unpin a clip.
pub async fn unpin_clip(server_id: Uuid, clip_id: Uuid, state: &AppState) -> AppResult<()> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(clip_id != Uuid::nil());

    sqlx::query("DELETE FROM pinned_clips WHERE server_id = $1 AND clip_id = $2")
        .bind(server_id)
        .bind(clip_id)
        .execute(state.db.pool())
        .await?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "clip_unpinned",
            "server_id": server_id,
            "clip_id": clip_id,
        }),
        state,
    )
    .await;

    Ok(())
}

// ═════════════════════════════════════════════════════════════════════════════
// EVENTS
// ═════════════════════════════════════════════════════════════════════════════

/// Create a canvas event (countdown).
pub async fn create_event(
    server_id: Uuid,
    user_id: Uuid,
    body: &CreateEventReq,
    state: &AppState,
) -> AppResult<CanvasEventResp> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let event_at: DateTime<Utc> = body
        .event_at
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid event_at timestamp".into()))?;

    if event_at <= Utc::now() {
        return Err(AppError::BadRequest(
            "event_at must be in the future".into(),
        ));
    }
    let max_future = Utc::now() + Duration::seconds(MAX_EVENT_FUTURE_SECS);
    if event_at > max_future {
        return Err(AppError::BadRequest(
            "event_at must be within 7 days".into(),
        ));
    }

    // One active event per server
    let live_window = Utc::now() - Duration::seconds(EVENT_LIVE_WINDOW_SECS);
    let has_active =
        sqlx::query("SELECT 1 FROM canvas_events WHERE server_id = $1 AND event_at > $2")
            .bind(server_id)
            .bind(live_window)
            .fetch_optional(state.db.pool())
            .await?;

    if has_active.is_some() {
        return Err(AppError::Conflict(
            "An active event already exists for this server".into(),
        ));
    }

    let row = sqlx::query(
        "INSERT INTO canvas_events (server_id, title, description, event_at, created_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, created_at",
    )
    .bind(server_id)
    .bind(&body.title)
    .bind(&body.description)
    .bind(event_at)
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await?;

    let event_id: Uuid = row.try_get("id")?;
    let created_at: DateTime<Utc> = row.try_get("created_at")?;

    let resp = CanvasEventResp {
        id: event_id,
        title: body.title.clone(),
        description: body.description.clone(),
        event_at,
        created_by: user_id,
        created_at,
    };

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "canvas_event_created",
            "server_id": server_id,
            "event": {
                "id": event_id,
                "title": body.title,
                "description": body.description,
                "event_at": event_at.to_rfc3339(),
                "created_by": user_id,
            },
        }),
        state,
    )
    .await;

    Ok(resp)
}

/// Update a canvas event.
pub async fn update_event(
    event_id: Uuid,
    user_id: Uuid,
    body: &UpdateEventReq,
    state: &AppState,
) -> AppResult<CanvasEventResp> {
    debug_assert!(event_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let existing = sqlx::query(
        "SELECT server_id, title, description, event_at, created_by, created_at
         FROM canvas_events WHERE id = $1",
    )
    .bind(event_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Event not found".into()))?;

    let server_id: Uuid = existing.try_get("server_id")?;
    require_server_admin(server_id, user_id, state).await?;

    // Validate event_at if provided
    let new_event_at: Option<DateTime<Utc>> = if let Some(ref ea) = body.event_at {
        let parsed: DateTime<Utc> = ea
            .parse()
            .map_err(|_| AppError::BadRequest("Invalid event_at".into()))?;
        if parsed <= Utc::now() {
            return Err(AppError::BadRequest(
                "event_at must be in the future".into(),
            ));
        }
        let max_future = Utc::now() + Duration::seconds(MAX_EVENT_FUTURE_SECS);
        if parsed > max_future {
            return Err(AppError::BadRequest(
                "event_at must be within 7 days".into(),
            ));
        }
        Some(parsed)
    } else {
        None
    };

    let row = sqlx::query(
        "UPDATE canvas_events SET
            title       = COALESCE($2, title),
            description = COALESCE($3, description),
            event_at    = COALESCE($4, event_at),
            updated_at  = NOW()
         WHERE id = $1
         RETURNING title, description, event_at, created_by, created_at",
    )
    .bind(event_id)
    .bind(&body.title)
    .bind(&body.description)
    .bind(new_event_at)
    .fetch_one(state.db.pool())
    .await?;

    let resp = CanvasEventResp {
        id: event_id,
        title: row.try_get("title")?,
        description: row.try_get("description").ok().flatten(),
        event_at: row.try_get("event_at")?,
        created_by: row.try_get("created_by")?,
        created_at: row.try_get("created_at")?,
    };

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "canvas_event_updated",
            "server_id": server_id,
            "event": {
                "id": event_id,
                "title": resp.title,
                "description": resp.description,
                "event_at": resp.event_at.to_rfc3339(),
                "created_by": resp.created_by,
            },
        }),
        state,
    )
    .await;

    Ok(resp)
}

/// Delete a canvas event.
pub async fn delete_event(event_id: Uuid, user_id: Uuid, state: &AppState) -> AppResult<()> {
    debug_assert!(event_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let row = sqlx::query("SELECT server_id FROM canvas_events WHERE id = $1")
        .bind(event_id)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("Event not found".into()))?;

    let server_id: Uuid = row.try_get("server_id")?;
    require_server_admin(server_id, user_id, state).await?;

    sqlx::query("DELETE FROM canvas_events WHERE id = $1")
        .bind(event_id)
        .execute(state.db.pool())
        .await?;

    broadcast_canvas(
        server_id,
        serde_json::json!({
            "type": "canvas_event_deleted",
            "server_id": server_id,
            "event_id": event_id,
        }),
        state,
    )
    .await;

    Ok(())
}

// ═════════════════════════════════════════════════════════════════════════════
// HYDRATION
// ═════════════════════════════════════════════════════════════════════════════

/// Fetch full canvas state for initial hydration.
pub async fn get_canvas_state(
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<serde_json::Value> {
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    // 1. Now playing
    let now_playing = fetch_now_playing(server_id, state).await?;

    // 2. Queue
    let queue = fetch_active_queue(server_id, state).await;

    // 3. Skip votes + settings
    let skip_votes: i64 =
        sqlx::query("SELECT COUNT(*) AS cnt FROM music_skip_votes WHERE server_id = $1")
            .bind(server_id)
            .fetch_one(state.db.pool())
            .await?
            .try_get("cnt")?;

    let threshold: i16 =
        sqlx::query("SELECT skip_threshold_pct FROM canvas_music_settings WHERE server_id = $1")
            .bind(server_id)
            .fetch_optional(state.db.pool())
            .await?
            .and_then(|r| r.try_get("skip_threshold_pct").ok())
            .unwrap_or(DEFAULT_SKIP_THRESHOLD_PCT);

    let member_ids = get_server_member_ids(server_id, state).await;
    let online_count = state.hub.count_online(&member_ids).max(1);

    // 4. Active polls for channel
    let polls = fetch_active_polls(channel_id, user_id, state).await?;

    // 5. Pinned clips
    let pinned_clip_rows = sqlx::query(
        "SELECT m.id, m.channel_id, m.sender_id, m.ciphertext, m.created_at, pc.pinned_at
         FROM pinned_clips pc
         JOIN messages m ON m.id = pc.clip_id
         WHERE pc.server_id = $1 AND m.deleted_at IS NULL
           AND m.created_at > NOW() - INTERVAL '30 days'
         ORDER BY pc.pinned_at DESC
         LIMIT $2",
    )
    .bind(server_id)
    .bind(MAX_PINNED_CLIPS)
    .fetch_all(state.db.pool())
    .await?;

    // 6. Recent clips (non-pinned)
    let clip_rows = sqlx::query(
        "SELECT m.id, m.channel_id, m.sender_id, m.ciphertext, m.created_at
         FROM messages m
         JOIN channels c ON c.id = m.channel_id
         WHERE c.server_id = $1
           AND m.message_type = 'clip'
           AND m.deleted_at IS NULL
           AND m.ciphertext IS NOT NULL
           AND m.created_at > NOW() - INTERVAL '30 days'
           AND m.id NOT IN (SELECT clip_id FROM pinned_clips WHERE server_id = $1)
         ORDER BY m.created_at DESC
         LIMIT 20",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    // Collect all clip IDs for batch reaction fetch
    let all_clip_ids: Vec<Uuid> = pinned_clip_rows
        .iter()
        .chain(clip_rows.iter())
        .filter_map(|r| r.try_get::<Uuid, _>("id").ok())
        .collect();

    // 7. Batch fetch reactions
    let reaction_map = fetch_batch_reaction_counts(&all_clip_ids, state).await?;
    let my_reactions = fetch_batch_my_reactions(&all_clip_ids, user_id, state).await?;

    // Build clip responses
    let pinned_clips = build_clip_list(&pinned_clip_rows, &reaction_map, &my_reactions, true);
    let clips = build_clip_list(&clip_rows, &reaction_map, &my_reactions, false);

    // 8. Active event
    let live_window = Utc::now() - Duration::seconds(EVENT_LIVE_WINDOW_SECS);
    let event = sqlx::query(
        "SELECT id, title, description, event_at, created_by, created_at
         FROM canvas_events
         WHERE server_id = $1 AND event_at > $2
         ORDER BY event_at DESC LIMIT 1",
    )
    .bind(server_id)
    .bind(live_window)
    .fetch_optional(state.db.pool())
    .await?
    .map(|r| -> Result<serde_json::Value, sqlx::Error> {
        Ok(serde_json::json!({
            "id": r.try_get::<Uuid, _>("id")?,
            "title": r.try_get::<String, _>("title")?,
            "description": r.try_get::<Option<String>, _>("description").ok().flatten(),
            "event_at": r.try_get::<DateTime<Utc>, _>("event_at")?
                         .to_rfc3339(),
            "created_by": r.try_get::<Uuid, _>("created_by")?,
            "created_at": r.try_get::<DateTime<Utc>, _>("created_at")?
                           .to_rfc3339(),
        }))
    })
    .transpose()?;

    Ok(serde_json::json!({
        "music": {
            "now_playing": now_playing,
            "queue": queue,
            "skip_votes": skip_votes,
            "skip_threshold_pct": threshold,
            "online_members": online_count,
        },
        "polls": polls,
        "pinned_clips": pinned_clips,
        "clips": clips,
        "event": event,
    }))
}

/// Fetch now-playing for a server.
async fn fetch_now_playing(
    server_id: Uuid,
    state: &AppState,
) -> AppResult<Option<serde_json::Value>> {
    let row = sqlx::query(
        "SELECT artist, title, duration_secs, source_url, album_art_url,
                set_by_user_id, started_at, current_track_id
         FROM canvas_music_state WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    Ok(row.map(|r| {
        serde_json::json!({
            "track_id": r.try_get::<Option<Uuid>, _>("current_track_id").ok().flatten(),
            "artist": r.try_get::<String, _>("artist").unwrap_or_default(),
            "title": r.try_get::<String, _>("title").unwrap_or_default(),
            "duration_secs": r.try_get::<Option<i32>, _>("duration_secs").ok().flatten(),
            "source_url": r.try_get::<Option<String>, _>("source_url").ok().flatten(),
            "album_art_url": r.try_get::<Option<String>, _>("album_art_url").ok().flatten(),
            "started_at": r.try_get::<DateTime<Utc>, _>("started_at")
                           .ok().map(|t| t.to_rfc3339()),
            "set_by": r.try_get::<Uuid, _>("set_by_user_id").ok(),
        })
    }))
}

/// Fetch active polls for a channel with the caller's vote.
async fn fetch_active_polls(
    channel_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> AppResult<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        "SELECT p.id, p.poll_type, p.question, p.options, p.ends_at, p.created_at,
                p.anonymous, p.status,
                (SELECT jsonb_object_agg(v.option_index::text, v.cnt)
                 FROM (SELECT option_index, COUNT(*) AS cnt
                       FROM poll_votes WHERE poll_id = p.id
                       GROUP BY option_index) v) AS vote_counts,
                pv.option_index AS my_vote
         FROM polls p
         LEFT JOIN poll_votes pv ON pv.poll_id = p.id AND pv.user_id = $2
         WHERE p.channel_id = $1
           AND p.status = 'active'
           AND (p.ends_at IS NULL OR p.ends_at > NOW())
         ORDER BY p.created_at DESC
         LIMIT $3",
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(MAX_ACTIVE_POLLS_PER_CHANNEL)
    .fetch_all(state.db.pool())
    .await?;

    rows.iter()
        .map(|r| -> Result<serde_json::Value, sqlx::Error> {
            Ok(serde_json::json!({
                "id": r.try_get::<Uuid, _>("id")?,
                "poll_type": r.try_get::<String, _>("poll_type")?,
                "question": r.try_get::<String, _>("question")?,
                "options": r.try_get::<serde_json::Value, _>("options")?,
                "vote_counts": r.try_get::<Option<serde_json::Value>, _>("vote_counts")
                                .ok().flatten().unwrap_or(serde_json::json!({})),
                "anonymous": r.try_get::<bool, _>("anonymous")?,
                "status": r.try_get::<String, _>("status")?,
                "ends_at": r.try_get::<Option<DateTime<Utc>>, _>("ends_at")
                            .ok().flatten().map(|t| t.to_rfc3339()),
                "created_at": r.try_get::<DateTime<Utc>, _>("created_at")?
                               .to_rfc3339(),
                "my_vote": r.try_get::<Option<i32>, _>("my_vote").ok().flatten(),
            }))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)
}

/// Batch fetch reaction counts for multiple clips.
async fn fetch_batch_reaction_counts(
    clip_ids: &[Uuid],
    state: &AppState,
) -> AppResult<std::collections::HashMap<Uuid, serde_json::Value>> {
    use std::collections::HashMap;

    if clip_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query(
        "SELECT clip_id, emoji, COUNT(*) AS cnt
         FROM clip_reactions WHERE clip_id = ANY($1)
         GROUP BY clip_id, emoji",
    )
    .bind(clip_ids)
    .fetch_all(state.db.pool())
    .await?;

    let mut map: HashMap<Uuid, serde_json::Map<String, serde_json::Value>> = HashMap::new();
    for r in &rows {
        let cid: Uuid = r.try_get("clip_id")?;
        let emoji: String = r.try_get("emoji")?;
        let cnt: i64 = r.try_get("cnt")?;
        map.entry(cid)
            .or_default()
            .insert(emoji, serde_json::json!(cnt));
    }

    Ok(map
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::Object(v)))
        .collect())
}

/// Batch fetch the caller's reactions for multiple clips.
async fn fetch_batch_my_reactions(
    clip_ids: &[Uuid],
    user_id: Uuid,
    state: &AppState,
) -> AppResult<std::collections::HashMap<Uuid, Vec<String>>> {
    use std::collections::HashMap;

    if clip_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query(
        "SELECT clip_id, emoji FROM clip_reactions
         WHERE clip_id = ANY($1) AND user_id = $2",
    )
    .bind(clip_ids)
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await?;

    let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for r in &rows {
        let cid: Uuid = r.try_get("clip_id")?;
        let emoji: String = r.try_get("emoji")?;
        map.entry(cid).or_default().push(emoji);
    }

    Ok(map)
}

/// Build clip JSON list from rows + reaction maps.
fn build_clip_list(
    rows: &[sqlx::postgres::PgRow],
    reaction_map: &std::collections::HashMap<Uuid, serde_json::Value>,
    my_reactions: &std::collections::HashMap<Uuid, Vec<String>>,
    include_pinned_at: bool,
) -> Vec<serde_json::Value> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    rows.iter()
        .filter_map(|r| {
            let id: Uuid = r.try_get("id").ok()?;
            let ciphertext = r
                .try_get::<Vec<u8>, _>("ciphertext")
                .ok()
                .map(|b| BASE64.encode(&b));

            let mut obj = serde_json::json!({
                "id": id,
                "channel_id": r.try_get::<Uuid, _>("channel_id").ok()?,
                "sender_id": r.try_get::<Uuid, _>("sender_id").ok()?,
                "ciphertext": ciphertext,
                "reactions": reaction_map.get(&id).cloned()
                    .unwrap_or(serde_json::json!({})),
                "my_reactions": my_reactions.get(&id).cloned()
                    .unwrap_or_default(),
                "created_at": r.try_get::<DateTime<Utc>, _>("created_at")
                               .ok()?.to_rfc3339(),
            });

            if include_pinned_at {
                obj["pinned_at"] = r
                    .try_get::<DateTime<Utc>, _>("pinned_at")
                    .ok()
                    .map(|t| serde_json::json!(t.to_rfc3339()))
                    .unwrap_or(serde_json::Value::Null);
            }

            Some(obj)
        })
        .collect()
}
