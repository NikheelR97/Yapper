use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            axum::routing::post(create_or_get_conversation).get(list_conversations),
        )
        .route("/:id/messages", get(list_messages))
}

// ─── Create or Get DM Conversation ───────────────────────────────────────────

#[derive(Deserialize)]
struct CreateConversationReq {
    peer_id: Uuid,
}

#[derive(Serialize)]
struct ConversationResp {
    id: Uuid,
    peer_id: Uuid,
    created_at: DateTime<Utc>,
}

async fn create_or_get_conversation(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateConversationReq>,
) -> AppResult<Json<ConversationResp>> {
    if req.peer_id == auth.user_id {
        return Err(AppError::BadRequest("Cannot DM yourself".into()));
    }

    // Check peer exists
    let peer_exists = sqlx::query(
        "SELECT 1 FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(req.peer_id)
    .fetch_optional(state.db.pool())
    .await?
    .is_some();

    if !peer_exists {
        return Err(AppError::NotFound("Peer user not found".into()));
    }

    // Look for existing conversation between the two users
    let existing = sqlx::query(
        r#"
        SELECT dp1.conversation_id AS id, dm.created_at
        FROM dm_participants dp1
        JOIN dm_participants dp2 ON dp1.conversation_id = dp2.conversation_id
        JOIN dm_conversations dm  ON dm.id = dp1.conversation_id
        WHERE dp1.user_id = $1 AND dp2.user_id = $2
        LIMIT 1
        "#,
    )
    .bind(auth.user_id)
    .bind(req.peer_id)
    .fetch_optional(state.db.pool())
    .await?;

    if let Some(row) = existing {
        let id: Uuid = row.try_get("id")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        return Ok(Json(ConversationResp {
            id,
            peer_id: req.peer_id,
            created_at,
        }));
    }

    // Create a new conversation
    let mut tx = state.db.pool().begin().await?;

    let conv_id: Uuid = sqlx::query(
        "INSERT INTO dm_conversations DEFAULT VALUES RETURNING id, created_at",
    )
    .fetch_one(&mut *tx)
    .await?
    .try_get("id")?;

    let created_at_row = sqlx::query("SELECT created_at FROM dm_conversations WHERE id = $1")
        .bind(conv_id)
        .fetch_one(&mut *tx)
        .await?;
    let created_at: DateTime<Utc> = created_at_row.try_get("created_at")?;

    sqlx::query(
        "INSERT INTO dm_participants (conversation_id, user_id) VALUES ($1, $2), ($1, $3)",
    )
    .bind(conv_id)
    .bind(auth.user_id)
    .bind(req.peer_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ConversationResp {
        id: conv_id,
        peer_id: req.peer_id,
        created_at,
    }))
}

// ─── List Conversations ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct ConversationListItem {
    id: Uuid,
    peer_id: Uuid,
    peer_username: String,
    peer_display_name: Option<String>,
    peer_avatar_url: Option<String>,
    last_message_at: Option<DateTime<Utc>>,
}

async fn list_conversations(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ConversationListItem>>> {
    let rows = sqlx::query(
        r#"
        SELECT
            dc.id,
            u.id         AS peer_id,
            u.username   AS peer_username,
            u.display_name AS peer_display_name,
            u.avatar_url AS peer_avatar_url,
            (
                SELECT m.created_at FROM messages m
                WHERE m.conversation_id = dc.id
                ORDER BY m.created_at DESC LIMIT 1
            ) AS last_message_at
        FROM dm_conversations dc
        JOIN dm_participants dp_me   ON dp_me.conversation_id   = dc.id AND dp_me.user_id   = $1
        JOIN dm_participants dp_peer ON dp_peer.conversation_id = dc.id AND dp_peer.user_id != $1
        JOIN users u ON u.id = dp_peer.user_id
        WHERE u.deleted_at IS NULL
        ORDER BY last_message_at DESC NULLS LAST
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let items = rows
        .into_iter()
        .map(|r| ConversationListItem {
            id: r.try_get("id").unwrap(),
            peer_id: r.try_get("peer_id").unwrap(),
            peer_username: r.try_get("peer_username").unwrap(),
            peer_display_name: r.try_get("peer_display_name").ok().flatten(),
            peer_avatar_url: r.try_get("peer_avatar_url").ok().flatten(),
            last_message_at: r.try_get("last_message_at").ok().flatten(),
        })
        .collect();

    Ok(Json(items))
}

// ─── List Messages (paginated, cursor-based) ──────────────────────────────────

#[derive(Deserialize)]
struct ListMessagesQuery {
    /// Fetch messages created before this cursor (UUID of last known message).
    before: Option<Uuid>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct MessageResp {
    id: Uuid,
    conversation_id: Uuid,
    sender_id: Uuid,
    /// Base64-encoded AES-256-GCM ciphertext.
    ciphertext: String,
    /// Base64-encoded X25519 ephemeral public key — only on first message of a session.
    ephemeral_key: Option<String>,
    /// OPK id used for X3DH — only on first message.
    opk_id: Option<i32>,
    msg_num: i64,
    created_at: DateTime<Utc>,
}

async fn list_messages(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Query(q): Query<ListMessagesQuery>,
) -> AppResult<Json<Vec<MessageResp>>> {
    // Verify caller is a participant
    let is_participant = sqlx::query(
        "SELECT 1 FROM dm_participants WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conv_id)
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .is_some();

    if !is_participant {
        return Err(AppError::Forbidden);
    }

    let limit = q.limit.unwrap_or(50).min(100);

    let rows = if let Some(before_id) = q.before {
        // Cursor-based pagination: messages older than `before`
        sqlx::query(
            r#"
            SELECT id, conversation_id, sender_id, ciphertext, ek_public, opk_id,
                   COALESCE(
                       (SELECT COUNT(*) FROM messages m2
                        WHERE m2.conversation_id = $1 AND m2.created_at < m.created_at),
                       0
                   ) AS msg_num,
                   created_at
            FROM messages m
            WHERE conversation_id = $1
              AND deleted_at IS NULL
              AND created_at < (SELECT created_at FROM messages WHERE id = $2)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(conv_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(state.db.pool())
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT id, conversation_id, sender_id, ciphertext, ek_public, opk_id,
                   COALESCE(
                       (SELECT COUNT(*) FROM messages m2
                        WHERE m2.conversation_id = $1 AND m2.created_at < m.created_at),
                       0
                   ) AS msg_num,
                   created_at
            FROM messages m
            WHERE conversation_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(conv_id)
        .bind(limit)
        .fetch_all(state.db.pool())
        .await?
    };

    let mut messages: Vec<MessageResp> = rows
        .into_iter()
        .map(|r| {
            let cipher: Vec<u8> = r.try_get("ciphertext").unwrap_or_default();
            let ek: Option<Vec<u8>> = r.try_get("ek_public").ok().flatten();
            MessageResp {
                id: r.try_get("id").unwrap(),
                conversation_id: r.try_get("conversation_id").unwrap(),
                sender_id: r.try_get("sender_id").unwrap(),
                ciphertext: BASE64.encode(&cipher),
                ephemeral_key: ek.as_ref().map(|k| BASE64.encode(k)),
                opk_id: r.try_get("opk_id").ok().flatten(),
                msg_num: r.try_get("msg_num").unwrap_or(0),
                created_at: r.try_get("created_at").unwrap(),
            }
        })
        .collect();

    // Return in chronological order
    messages.reverse();
    Ok(Json(messages))
}
