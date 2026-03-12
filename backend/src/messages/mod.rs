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
    auth::{AuthDevice, AuthUser},
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

pub fn v2_router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            axum::routing::post(create_or_get_conversation_v2).get(list_conversations_v2),
        )
        .route("/:id/messages", get(list_messages_v2).post(send_message_v2))
}

// ─── Create or Get DM Conversation ───────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
    create_or_get_conversation_for_user(auth.user_id, req, state).await
}

async fn create_or_get_conversation_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(req): Json<CreateConversationReq>,
) -> AppResult<Json<ConversationResp>> {
    auth.require_trusted()?;
    create_or_get_conversation_for_user(auth.user_id, req, state).await
}

async fn create_or_get_conversation_for_user(
    user_id: Uuid,
    req: CreateConversationReq,
    state: AppState,
) -> AppResult<Json<ConversationResp>> {
    if req.peer_id == user_id {
        return Err(AppError::BadRequest("Cannot DM yourself".into()));
    }

    // Check peer exists
    let peer_exists = sqlx::query("SELECT 1 FROM users WHERE id = $1 AND deleted_at IS NULL")
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
    .bind(user_id)
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

    let conv_id: Uuid =
        sqlx::query("INSERT INTO dm_conversations DEFAULT VALUES RETURNING id, created_at")
            .fetch_one(&mut *tx)
            .await?
            .try_get("id")?;

    let created_at_row = sqlx::query("SELECT created_at FROM dm_conversations WHERE id = $1")
        .bind(conv_id)
        .fetch_one(&mut *tx)
        .await?;
    let created_at: DateTime<Utc> = created_at_row.try_get("created_at")?;

    sqlx::query("INSERT INTO dm_participants (conversation_id, user_id) VALUES ($1, $2), ($1, $3)")
        .bind(conv_id)
        .bind(user_id)
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
    list_conversations_for_user(auth.user_id, state).await
}

async fn list_conversations_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ConversationListItem>>> {
    auth.require_trusted()?;
    list_conversations_for_user(auth.user_id, state).await
}

async fn list_conversations_for_user(
    user_id: Uuid,
    state: AppState,
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
    .bind(user_id)
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
#[serde(deny_unknown_fields)]
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

#[derive(Serialize)]
struct MessageRespV2 {
    id: Uuid,
    conversation_id: Uuid,
    sender_id: Uuid,
    sender_device_id: Uuid,
    sender_signal_device_id: i32,
    ciphertext: String,
    ephemeral_key: Option<String>,
    opk_id: Option<i32>,
    msg_num: i32,
    ratchet_pub: Option<String>,
    previous_chain_len: Option<i32>,
    crypto_version: i16,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendEnvelopeReqV2 {
    recipient_user_id: Uuid,
    recipient_device_id: Uuid,
    ciphertext: String,
    ephemeral_key: Option<String>,
    opk_id: Option<i32>,
    msg_num: i32,
    ratchet_pub: Option<String>,
    previous_chain_len: Option<i32>,
    crypto_version: Option<i16>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendMessageReqV2 {
    envelopes: Vec<SendEnvelopeReqV2>,
}

async fn list_messages(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Query(q): Query<ListMessagesQuery>,
) -> AppResult<Json<Vec<MessageResp>>> {
    list_messages_v1_for_user(auth.user_id, conv_id, q, state).await
}

async fn list_messages_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Query(q): Query<ListMessagesQuery>,
) -> AppResult<Json<Vec<MessageRespV2>>> {
    auth.require_trusted()?;

    let is_participant =
        sqlx::query("SELECT 1 FROM dm_participants WHERE conversation_id = $1 AND user_id = $2")
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
        sqlx::query(
            r#"
            SELECT m.id,
                   m.conversation_id,
                   m.sender_id,
                   m.sender_device_id,
                   sd.signal_device_id AS sender_signal_device_id,
                   e.ciphertext,
                   e.ek_public,
                   e.opk_id,
                   e.msg_num,
                   e.ratchet_pub,
                   e.previous_chain_len,
                   e.crypto_version,
                   m.created_at
            FROM dm_message_envelopes e
            JOIN messages m ON m.id = e.message_id
            JOIN devices sd ON sd.id = m.sender_device_id
            WHERE m.conversation_id = $1
              AND e.recipient_device_id = $2
              AND m.deleted_at IS NULL
              AND m.created_at < (SELECT created_at FROM messages WHERE id = $3)
            ORDER BY m.created_at DESC
            LIMIT $4
            "#,
        )
        .bind(conv_id)
        .bind(auth.device_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(state.db.pool())
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT m.id,
                   m.conversation_id,
                   m.sender_id,
                   m.sender_device_id,
                   sd.signal_device_id AS sender_signal_device_id,
                   e.ciphertext,
                   e.ek_public,
                   e.opk_id,
                   e.msg_num,
                   e.ratchet_pub,
                   e.previous_chain_len,
                   e.crypto_version,
                   m.created_at
            FROM dm_message_envelopes e
            JOIN messages m ON m.id = e.message_id
            JOIN devices sd ON sd.id = m.sender_device_id
            WHERE m.conversation_id = $1
              AND e.recipient_device_id = $2
              AND m.deleted_at IS NULL
            ORDER BY m.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(conv_id)
        .bind(auth.device_id)
        .bind(limit)
        .fetch_all(state.db.pool())
        .await?
    };

    let mut messages: Vec<MessageRespV2> = rows
        .into_iter()
        .map(|r| {
            let cipher: Vec<u8> = r.try_get("ciphertext").unwrap_or_default();
            let ek: Option<Vec<u8>> = r.try_get("ek_public").ok().flatten();
            let ratchet_pub: Option<Vec<u8>> = r.try_get("ratchet_pub").ok().flatten();
            MessageRespV2 {
                id: r.try_get("id").unwrap(),
                conversation_id: r.try_get("conversation_id").unwrap(),
                sender_id: r.try_get("sender_id").unwrap(),
                sender_device_id: r.try_get("sender_device_id").unwrap(),
                sender_signal_device_id: r.try_get("sender_signal_device_id").unwrap(),
                ciphertext: BASE64.encode(&cipher),
                ephemeral_key: ek.as_ref().map(|k| BASE64.encode(k)),
                opk_id: r.try_get("opk_id").ok().flatten(),
                msg_num: r.try_get("msg_num").unwrap_or(0),
                ratchet_pub: ratchet_pub.as_ref().map(|key| BASE64.encode(key)),
                previous_chain_len: r.try_get("previous_chain_len").ok().flatten(),
                crypto_version: r.try_get("crypto_version").unwrap_or(1),
                created_at: r.try_get("created_at").unwrap(),
            }
        })
        .collect();

    messages.reverse();
    Ok(Json(messages))
}

async fn list_messages_v1_for_user(
    user_id: Uuid,
    conv_id: Uuid,
    q: ListMessagesQuery,
    state: AppState,
) -> AppResult<Json<Vec<MessageResp>>> {
    // Verify caller is a participant
    let is_participant =
        sqlx::query("SELECT 1 FROM dm_participants WHERE conversation_id = $1 AND user_id = $2")
            .bind(conv_id)
            .bind(user_id)
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

async fn send_message_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Json(req): Json<SendMessageReqV2>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_trusted()?;

    if req.envelopes.is_empty() || req.envelopes.len() > 16 {
        return Err(AppError::BadRequest(
            "Provide between 1 and 16 DM envelopes".into(),
        ));
    }

    let participant_rows = sqlx::query(
        "SELECT user_id FROM dm_participants WHERE conversation_id = $1 ORDER BY user_id ASC",
    )
    .bind(conv_id)
    .fetch_all(state.db.pool())
    .await?;

    let participants: Vec<Uuid> = participant_rows
        .iter()
        .filter_map(|row| row.try_get::<Uuid, _>("user_id").ok())
        .collect();

    if participants.len() != 2 || !participants.contains(&auth.user_id) {
        return Err(AppError::Forbidden);
    }

    let mut recipient_user_ids = req
        .envelopes
        .iter()
        .map(|envelope| envelope.recipient_user_id)
        .collect::<Vec<_>>();
    recipient_user_ids.sort();
    recipient_user_ids.dedup();

    if recipient_user_ids
        .iter()
        .any(|user_id| !participants.contains(user_id))
    {
        return Err(AppError::BadRequest(
            "Envelope recipients must belong to the conversation".into(),
        ));
    }

    let recipient_device_ids = req
        .envelopes
        .iter()
        .map(|envelope| envelope.recipient_device_id)
        .collect::<Vec<_>>();

    let device_rows = sqlx::query(
        r#"
        SELECT id, user_id, revoked_at, trust_state
        FROM devices
        WHERE id = ANY($1)
        "#,
    )
    .bind(&recipient_device_ids)
    .fetch_all(state.db.pool())
    .await?;

    if device_rows.len() != req.envelopes.len() {
        return Err(AppError::BadRequest("Unknown recipient device".into()));
    }

    let mut device_map = std::collections::HashMap::new();
    for row in &device_rows {
        let device_id: Uuid = row.try_get("id")?;
        let user_id: Uuid = row.try_get("user_id")?;
        let revoked_at: Option<DateTime<Utc>> = row.try_get("revoked_at").ok().flatten();
        let trust_state: String = row.try_get("trust_state")?;
        if revoked_at.is_some() || trust_state != "trusted" {
            return Err(AppError::BadRequest(
                "Recipient device must be trusted and active".into(),
            ));
        }
        device_map.insert(device_id, user_id);
    }

    for envelope in &req.envelopes {
        let owner = device_map
            .get(&envelope.recipient_device_id)
            .copied()
            .ok_or_else(|| AppError::BadRequest("Unknown recipient device".into()))?;
        if owner != envelope.recipient_user_id {
            return Err(AppError::BadRequest(
                "Recipient device does not belong to recipient user".into(),
            ));
        }
    }

    let message_id = Uuid::new_v4();
    let created_at = Utc::now();
    let mut tx = state.db.pool().begin().await?;

    sqlx::query(
        r#"
        INSERT INTO messages
            (id, conversation_id, sender_id, sender_device_id, delivered, created_at)
        VALUES ($1, $2, $3, $4, TRUE, $5)
        "#,
    )
    .bind(message_id)
    .bind(conv_id)
    .bind(auth.user_id)
    .bind(auth.device_id)
    .bind(created_at)
    .execute(&mut *tx)
    .await?;
    let mut delivered_device_ids = Vec::new();

    for envelope in &req.envelopes {
        let ciphertext = BASE64
            .decode(&envelope.ciphertext)
            .map_err(|_| AppError::BadRequest("Invalid ciphertext encoding".into()))?;
        let ek_public = envelope
            .ephemeral_key
            .as_deref()
            .map(|value| BASE64.decode(value))
            .transpose()
            .map_err(|_| AppError::BadRequest("Invalid ephemeral_key encoding".into()))?;
        let ratchet_pub = envelope
            .ratchet_pub
            .as_deref()
            .map(|value| BASE64.decode(value))
            .transpose()
            .map_err(|_| AppError::BadRequest("Invalid ratchet_pub encoding".into()))?;
        let crypto_version = envelope.crypto_version.unwrap_or(1);
        if !(1..=2).contains(&crypto_version) {
            return Err(AppError::BadRequest("Invalid crypto_version".into()));
        }

        let deliver_now = envelope.recipient_device_id == auth.device_id
            || state.hub.is_device_online(&envelope.recipient_device_id);

        sqlx::query(
            r#"
            INSERT INTO dm_message_envelopes
                (message_id, recipient_user_id, recipient_device_id, sender_user_id, sender_device_id,
                 ciphertext, ek_public, opk_id, msg_num, ratchet_pub, previous_chain_len, crypto_version,
                 delivered_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(message_id)
        .bind(envelope.recipient_user_id)
        .bind(envelope.recipient_device_id)
        .bind(auth.user_id)
        .bind(auth.device_id)
        .bind(&ciphertext)
        .bind(&ek_public)
        .bind(envelope.opk_id)
        .bind(envelope.msg_num)
        .bind(&ratchet_pub)
        .bind(envelope.previous_chain_len)
        .bind(crypto_version)
        .bind(if deliver_now { Some(created_at) } else { None })
        .bind(created_at)
        .execute(&mut *tx)
        .await?;

        if deliver_now {
            delivered_device_ids.push(envelope.recipient_device_id);
        }
    }

    tx.commit().await?;

    for envelope in &req.envelopes {
        if envelope.recipient_device_id == auth.device_id {
            continue;
        }
        if !delivered_device_ids.contains(&envelope.recipient_device_id) {
            continue;
        }

        let payload = serde_json::json!({
            "type": "dm_v2",
            "id": message_id,
            "conversation_id": conv_id,
            "sender_id": auth.user_id,
            "sender_device_id": auth.device_id,
            "sender_signal_device_id": auth.signal_device_id,
            "recipient_device_id": envelope.recipient_device_id,
            "ciphertext": envelope.ciphertext,
            "ephemeral_key": envelope.ephemeral_key,
            "opk_id": envelope.opk_id,
            "msg_num": envelope.msg_num,
            "ratchet_pub": envelope.ratchet_pub,
            "previous_chain_len": envelope.previous_chain_len,
            "crypto_version": envelope.crypto_version.unwrap_or(1),
            "created_at": created_at,
        });
        state.hub.send_to_device(
            &envelope.recipient_device_id,
            crate::hub::WsOutbound::Message { payload },
        );
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message_id": message_id,
        "created_at": created_at,
    })))
}
