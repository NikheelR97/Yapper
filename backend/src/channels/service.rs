use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    AppState,
};
use super::handlers::ChannelResp;

/// Max server members to fan out a channel message to in a single operation.
const MAX_FANOUT_MEMBERS: usize = 500;

// ─── Message record returned by service ──────────────────────────────────────

pub struct MessageRecord {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub ciphertext: Option<String>,
    pub plaintext: Option<String>,
    pub ephemeral_key: Option<String>,
    pub opk_id: Option<i32>,
    pub message_type: String,
    pub msg_num: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ─── Membership helpers (pub — used by servers::service) ─────────────────────

pub async fn require_member(state: &AppState, user_id: Uuid, server_id: Uuid) -> AppResult<()> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    let exists = sqlx::query(
        "SELECT 1 FROM server_memberships WHERE user_id = $1 AND server_id = $2",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    if exists.is_none() {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn require_admin(state: &AppState, user_id: Uuid, server_id: Uuid) -> AppResult<()> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    let row = sqlx::query(
        "SELECT role FROM server_memberships WHERE user_id = $1 AND server_id = $2",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await?;

    match row {
        Some(r) => {
            let role: String = r.try_get("role")?;
            if role == "owner" || role == "admin" {
                Ok(())
            } else {
                Err(AppError::Forbidden)
            }
        }
        None => Err(AppError::Forbidden),
    }
}

/// Resolve the server_id that owns a channel.
pub async fn channel_server_id(state: &AppState, channel_id: Uuid) -> AppResult<Uuid> {
    debug_assert!(channel_id != Uuid::nil());

    let row = sqlx::query("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("Channel not found".into()))?;
    Ok(row.try_get("server_id")?)
}

// ─── Channel CRUD ─────────────────────────────────────────────────────────────

pub async fn list_channels(
    user_id: Uuid,
    server_id: Uuid,
    state: &AppState,
) -> AppResult<Vec<ChannelResp>> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    require_member(state, user_id, server_id).await?;

    let rows = sqlx::query(
        "SELECT id, server_id, name, type, position \
         FROM channels \
         WHERE server_id = $1 \
         ORDER BY position ASC",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    rows.iter()
        .map(|r| {
            Ok(ChannelResp {
                id: r.try_get("id")?,
                server_id: r.try_get("server_id")?,
                name: r.try_get("name")?,
                channel_type: r.try_get("type")?,
                position: r.try_get("position")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)
}

pub async fn create_channel(
    user_id: Uuid,
    server_id: Uuid,
    name: String,
    channel_type: &str,
    state: &AppState,
) -> AppResult<ChannelResp> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());
    debug_assert!(!name.is_empty(), "name must be non-empty (validated by handler)");

    require_admin(state, user_id, server_id).await?;

    let pos_row = sqlx::query(
        "SELECT COALESCE(MAX(position), -1) + 1 AS next_pos \
         FROM channels WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(state.db.pool())
    .await?;
    let position: i32 = pos_row.try_get("next_pos")?;

    let row = sqlx::query(
        "INSERT INTO channels (server_id, name, type, position) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, server_id, name, type, position",
    )
    .bind(server_id)
    .bind(&name)
    .bind(channel_type)
    .bind(position)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::Conflict(format!("Channel '{}' already exists", name))
        } else {
            AppError::Database(e)
        }
    })?;

    Ok(ChannelResp {
        id: row.try_get("id")?,
        server_id: row.try_get("server_id")?,
        name: row.try_get("name")?,
        channel_type: row.try_get("type")?,
        position: row.try_get("position")?,
    })
}

// ─── Messages ─────────────────────────────────────────────────────────────────

pub async fn get_messages(
    user_id: Uuid,
    channel_id: Uuid,
    before: Option<Uuid>,
    limit: i64,
    state: &AppState,
) -> AppResult<Vec<MessageRecord>> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(limit > 0 && limit <= 100);

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, user_id, server_id).await?;

    let rows = if let Some(before_id) = before {
        sqlx::query(
            "SELECT id, channel_id, sender_id, ciphertext, plaintext, ek_public, opk_id, \
                     message_type, msg_num, created_at \
             FROM messages \
             WHERE channel_id = $1 AND deleted_at IS NULL \
               AND created_at < (SELECT created_at FROM messages WHERE id = $2) \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(channel_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(state.db.pool())
        .await?
    } else {
        sqlx::query(
            "SELECT id, channel_id, sender_id, ciphertext, plaintext, ek_public, opk_id, \
                     message_type, msg_num, created_at \
             FROM messages \
             WHERE channel_id = $1 AND deleted_at IS NULL \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(channel_id)
        .bind(limit)
        .fetch_all(state.db.pool())
        .await?
    };

    let mut msgs = rows
        .iter()
        .map(|r| {
            let ct: Option<Vec<u8>> = r.try_get("ciphertext")?;
            let ek: Option<Vec<u8>> = r.try_get("ek_public")?;
            Ok(MessageRecord {
                id: r.try_get("id")?,
                channel_id: r.try_get("channel_id")?,
                sender_id: r.try_get("sender_id")?,
                ciphertext: ct.map(|b| BASE64.encode(&b)),
                plaintext: r.try_get("plaintext")?,
                ephemeral_key: ek.map(|b| BASE64.encode(&b)),
                opk_id: r.try_get("opk_id")?,
                message_type: r.try_get("message_type")?,
                msg_num: r.try_get("msg_num")?,
                created_at: r.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)?;

    msgs.reverse(); // Return in chronological order
    Ok(msgs)
}

#[allow(clippy::too_many_arguments)]
pub async fn send_message(
    user_id: Uuid,
    channel_id: Uuid,
    ciphertext_b64: String,
    ephemeral_key_b64: Option<String>,
    opk_id: Option<i32>,
    message_type: &str,
    msg_num: Option<i32>,
    state: &AppState,
) -> AppResult<MessageRecord> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(!ciphertext_b64.is_empty(), "ciphertext must not be empty");

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, user_id, server_id).await?;

    let ciphertext = BASE64
        .decode(&ciphertext_b64)
        .map_err(|_| AppError::BadRequest("Invalid ciphertext encoding".into()))?;

    let ek_bytes = ephemeral_key_b64
        .as_deref()
        .map(|s| BASE64.decode(s))
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid ephemeral_key encoding".into()))?;

    let row = sqlx::query(
        "INSERT INTO messages \
             (channel_id, sender_id, ciphertext, ek_public, opk_id, message_type, msg_num, delivered) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE) \
         RETURNING id, created_at",
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(&ciphertext)
    .bind(&ek_bytes)
    .bind(opk_id)
    .bind(message_type)
    .bind(msg_num)
    .fetch_one(state.db.pool())
    .await?;

    let msg_id: Uuid = row.try_get("id")?;
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;

    fanout_to_members(
        user_id, channel_id, server_id, msg_id, created_at,
        &ciphertext_b64, ephemeral_key_b64.as_deref(), opk_id, message_type, msg_num,
        state,
    )
    .await?;

    Ok(MessageRecord {
        id: msg_id,
        channel_id,
        sender_id: user_id,
        ciphertext: Some(ciphertext_b64),
        plaintext: None,
        ephemeral_key: ephemeral_key_b64,
        opk_id,
        message_type: message_type.to_string(),
        msg_num,
        created_at,
    })
}

#[allow(clippy::too_many_arguments)]
async fn fanout_to_members(
    sender_id: Uuid,
    channel_id: Uuid,
    server_id: Uuid,
    msg_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    ciphertext_b64: &str,
    ephemeral_key: Option<&str>,
    opk_id: Option<i32>,
    message_type: &str,
    msg_num: Option<i32>,
    state: &AppState,
) -> AppResult<()> {
    let member_rows = sqlx::query(
        "SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2",
    )
    .bind(server_id)
    .bind(MAX_FANOUT_MEMBERS as i64)
    .fetch_all(state.db.pool())
    .await?;

    let ws_payload = serde_json::json!({
        "id": msg_id,
        "channel_id": channel_id,
        "server_id": server_id,
        "sender_id": sender_id,
        "ciphertext": ciphertext_b64,
        "ephemeral_key": ephemeral_key,
        "opk_id": opk_id,
        "message_type": message_type,
        "msg_num": msg_num,
        "created_at": created_at,
    });

    for m in member_rows.iter().take(MAX_FANOUT_MEMBERS) {
        let uid: Uuid = m.try_get("user_id")?;
        if uid != sender_id {
            state.hub.send_to_user(
                &uid,
                crate::hub::WsOutbound::Message { payload: ws_payload.clone() },
            );
        }
    }

    Ok(())
}
