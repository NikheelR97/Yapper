use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthDevice,
    constants,
    error::{AppError, AppResult},
    users, AppState,
};

use super::types::*;

pub async fn create_or_get_conversation_for_user(
    user_id: Uuid,
    req: CreateConversationReq,
    state: AppState,
) -> AppResult<axum::Json<ConversationResp>> {
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

    let (user_low, user_high) = if user_id < req.peer_id {
        (user_id, req.peer_id)
    } else {
        (req.peer_id, user_id)
    };

    // Resolve through the canonical pair table first so duplicate DM threads
    // cannot be created for the same unordered user pair.
    let existing = sqlx::query(
        r#"
        SELECT dcp.conversation_id AS id, dc.created_at
        FROM dm_conversation_pairs dcp
        JOIN dm_conversations dc ON dc.id = dcp.conversation_id
        WHERE dcp.user_low = $1 AND dcp.user_high = $2
        "#,
    )
    .bind(user_low)
    .bind(user_high)
    .fetch_optional(state.db.pool())
    .await?;

    if let Some(row) = existing {
        let id: Uuid = row.try_get("id")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        return Ok(axum::Json(ConversationResp {
            id,
            peer_id: req.peer_id,
            created_at,
        }));
    }

    users::require_dm_access(user_id, req.peer_id, &state).await?;

    // Create a new conversation
    let mut tx = state.db.pool().begin().await?;

    let conv_row =
        sqlx::query("INSERT INTO dm_conversations DEFAULT VALUES RETURNING id, created_at")
            .fetch_one(&mut *tx)
            .await?;
    let conv_id: Uuid = conv_row.try_get("id")?;
    let created_at: DateTime<Utc> = conv_row.try_get("created_at")?;

    let pair_inserted = sqlx::query(
        r#"
        INSERT INTO dm_conversation_pairs (conversation_id, user_low, user_high)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_low, user_high) DO NOTHING
        RETURNING conversation_id
        "#,
    )
    .bind(conv_id)
    .bind(user_low)
    .bind(user_high)
    .fetch_optional(&mut *tx)
    .await?;

    if pair_inserted.is_none() {
        let existing = sqlx::query(
            r#"
            SELECT dcp.conversation_id AS id, dc.created_at
            FROM dm_conversation_pairs dcp
            JOIN dm_conversations dc ON dc.id = dcp.conversation_id
            WHERE dcp.user_low = $1 AND dcp.user_high = $2
            "#,
        )
        .bind(user_low)
        .bind(user_high)
        .fetch_one(&mut *tx)
        .await?;

        tx.rollback().await.ok();
        return Ok(axum::Json(ConversationResp {
            id: existing.try_get("id")?,
            peer_id: req.peer_id,
            created_at: existing.try_get("created_at")?,
        }));
    }

    sqlx::query("INSERT INTO dm_participants (conversation_id, user_id) VALUES ($1, $2), ($1, $3)")
        .bind(conv_id)
        .bind(user_id)
        .bind(req.peer_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(axum::Json(ConversationResp {
        id: conv_id,
        peer_id: req.peer_id,
        created_at,
    }))
}

pub async fn list_conversations_for_user(
    user_id: Uuid,
    state: AppState,
) -> AppResult<axum::Json<Vec<ConversationListItem>>> {
    let rows = sqlx::query(
        r#"
        SELECT
            dc.id,
            u.id         AS peer_id,
            u.username   AS peer_username,
            u.display_name AS peer_display_name,
            u.avatar_url AS peer_avatar_url,
            dc.last_message_at
        FROM dm_conversations dc
        JOIN dm_participants dp_me   ON dp_me.conversation_id   = dc.id AND dp_me.user_id   = $1
        JOIN dm_participants dp_peer ON dp_peer.conversation_id = dc.id AND dp_peer.user_id != $1
        JOIN users u ON u.id = dp_peer.user_id
        WHERE u.deleted_at IS NULL
        ORDER BY dc.last_message_at DESC NULLS LAST
        "#,
    )
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await?;

    let items = rows
        .into_iter()
        .map(|r| -> Result<ConversationListItem, sqlx::Error> {
            Ok(ConversationListItem {
                id: r.try_get("id")?,
                peer_id: r.try_get("peer_id")?,
                peer_username: r.try_get("peer_username")?,
                peer_display_name: r.try_get("peer_display_name").ok().flatten(),
                peer_avatar_url: r.try_get("peer_avatar_url").ok().flatten(),
                last_message_at: r.try_get("last_message_at").ok().flatten(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(axum::Json(items))
}

/// Validate the sender is a conversation participant and all envelope
/// recipients belong to the conversation.
pub async fn validate_dm_participants(
    conv_id: Uuid,
    sender_id: Uuid,
    req: &SendMessageReqV2,
    state: &AppState,
) -> AppResult<()> {
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

    if participants.len() != 2 || !participants.contains(&sender_id) {
        return Err(AppError::Forbidden);
    }

    let all_recipients_valid = req
        .envelopes
        .iter()
        .all(|e| participants.contains(&e.recipient_user_id));

    if !all_recipients_valid {
        return Err(AppError::BadRequest(
            "Envelope recipients must belong to the conversation".into(),
        ));
    }
    Ok(())
}

/// Validate all recipient devices exist, are trusted, and belong to the
/// correct user IDs specified in each envelope.
pub async fn validate_dm_recipient_devices(
    req: &SendMessageReqV2,
    state: &AppState,
) -> AppResult<()> {
    let device_ids: Vec<Uuid> = req
        .envelopes
        .iter()
        .map(|e| e.recipient_device_id)
        .collect();
    let rows =
        sqlx::query("SELECT id, user_id, revoked_at, trust_state FROM devices WHERE id = ANY($1)")
            .bind(&device_ids)
            .fetch_all(state.db.pool())
            .await?;

    if rows.len() != req.envelopes.len() {
        return Err(AppError::BadRequest("Unknown recipient device".into()));
    }

    let mut device_map = std::collections::HashMap::new();
    for row in &rows {
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
    Ok(())
}

/// Insert the parent message row + per-device envelopes inside a transaction.
pub async fn store_dm_envelopes(
    message_id: Uuid,
    conv_id: Uuid,
    created_at: DateTime<Utc>,
    auth: &AuthDevice,
    req: &SendMessageReqV2,
    state: &AppState,
) -> AppResult<()> {
    let mut tx = state.db.pool().begin().await?;

    sqlx::query(
        "INSERT INTO messages (id, conversation_id, sender_id, sender_device_id, delivered, created_at) \
         VALUES ($1, $2, $3, $4, FALSE, $5)",
    )
    .bind(message_id).bind(conv_id).bind(auth.user_id).bind(auth.device_id).bind(created_at)
    .execute(&mut *tx)
    .await?;

    for envelope in &req.envelopes {
        insert_dm_envelope(message_id, created_at, auth, envelope, &mut tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Insert a single dm_message_envelope row.
async fn insert_dm_envelope(
    message_id: Uuid,
    created_at: DateTime<Utc>,
    auth: &AuthDevice,
    e: &SendEnvelopeReqV2,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> AppResult<()> {
    let ciphertext = BASE64
        .decode(&e.ciphertext)
        .map_err(|_| AppError::BadRequest("Invalid ciphertext encoding".into()))?;
    // Server-side E2EE enforcement is on ciphertext wire bytes, not plaintext chars.
    if ciphertext.is_empty() || ciphertext.len() > constants::MAX_MESSAGE_LENGTH {
        return Err(AppError::BadRequest("Ciphertext exceeds size limit".into()));
    }
    let ek_public = e
        .ephemeral_key
        .as_deref()
        .map(|v| BASE64.decode(v))
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid ephemeral_key encoding".into()))?;
    let ratchet_pub = e
        .ratchet_pub
        .as_deref()
        .map(|v| BASE64.decode(v))
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid ratchet_pub encoding".into()))?;
    let crypto_version = e.crypto_version.unwrap_or(1);
    if !(1..=2).contains(&crypto_version) {
        return Err(AppError::BadRequest("Invalid crypto_version".into()));
    }

    sqlx::query(
        "INSERT INTO dm_message_envelopes \
         (message_id, recipient_user_id, recipient_device_id, sender_user_id, sender_device_id, \
          ciphertext, ek_public, opk_id, msg_num, ratchet_pub, previous_chain_len, crypto_version, \
          delivered_at, created_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(message_id)
    .bind(e.recipient_user_id)
    .bind(e.recipient_device_id)
    .bind(auth.user_id)
    .bind(auth.device_id)
    .bind(&ciphertext)
    .bind(&ek_public)
    .bind(e.opk_id)
    .bind(e.msg_num)
    .bind(&ratchet_pub)
    .bind(e.previous_chain_len)
    .bind(crypto_version)
    .bind(None::<DateTime<Utc>>)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Push WS payloads to online recipient devices.
pub fn fanout_dm_v2(
    message_id: Uuid,
    conv_id: Uuid,
    created_at: DateTime<Utc>,
    auth: &AuthDevice,
    req: &SendMessageReqV2,
    state: &AppState,
) -> Vec<(Uuid, Uuid)> {
    let mut delivered = Vec::new();
    for envelope in &req.envelopes {
        if envelope.recipient_device_id == auth.device_id {
            continue;
        }
        if !state.hub.try_send_to_device(
            &envelope.recipient_device_id,
            crate::hub::WsOutbound::Message {
                payload: serde_json::json!({
                    "type": "dm_v2", "id": message_id, "conversation_id": conv_id,
                    "sender_id": auth.user_id, "sender_device_id": auth.device_id,
                    "sender_signal_device_id": auth.signal_device_id,
                    "recipient_device_id": envelope.recipient_device_id,
                    "ciphertext": envelope.ciphertext, "ephemeral_key": envelope.ephemeral_key,
                    "opk_id": envelope.opk_id, "msg_num": envelope.msg_num,
                    "ratchet_pub": envelope.ratchet_pub, "previous_chain_len": envelope.previous_chain_len,
                    "crypto_version": envelope.crypto_version.unwrap_or(1), "created_at": created_at,
                }),
            },
        ) {
            continue;
        }
        delivered.push((message_id, envelope.recipient_device_id));
    }
    delivered
}

