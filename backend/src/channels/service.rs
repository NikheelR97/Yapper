use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;
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

// ─── Channel members ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ChannelMember {
    pub user_id: Uuid,
    pub username: String,
}

pub async fn list_channel_members(
    user_id: Uuid,
    channel_id: Uuid,
    state: &AppState,
) -> AppResult<Vec<ChannelMember>> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, user_id, server_id).await?;

    let rows = sqlx::query(
        "SELECT u.id AS user_id, u.username \
         FROM server_memberships sm \
         JOIN users u ON u.id = sm.user_id \
         WHERE sm.server_id = $1 AND u.deleted_at IS NULL \
         ORDER BY sm.joined_at ASC \
         LIMIT 500",
    )
    .bind(server_id)
    .fetch_all(state.db.pool())
    .await?;

    rows.iter()
        .map(|r| Ok(ChannelMember { user_id: r.try_get("user_id")?, username: r.try_get("username")? }))
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)
}

// ─── Sender Key distributions ─────────────────────────────────────────────────

pub struct KeyDistItem {
    pub to_user: Uuid,
    pub ciphertext: Vec<u8>,
    pub ek_public: Vec<u8>,
}

#[derive(Serialize)]
pub struct KeyDistRecord {
    pub from_user: Uuid,
    pub ciphertext: String, // base64
    pub ek_public: String,  // base64
}

/// Upsert sender key distributions for a channel. Pushes to online recipients immediately.
pub async fn store_key_distributions(
    from_user: Uuid,
    channel_id: Uuid,
    items: Vec<KeyDistItem>,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(from_user != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, from_user, server_id).await?;

    for item in &items {
        sqlx::query(
            "INSERT INTO sender_key_distributions \
                 (channel_id, from_user, to_user, ciphertext, ek_public, delivered) \
             VALUES ($1, $2, $3, $4, $5, FALSE) \
             ON CONFLICT (channel_id, from_user, to_user) DO UPDATE \
                 SET ciphertext = EXCLUDED.ciphertext, \
                     ek_public  = EXCLUDED.ek_public, \
                     delivered  = FALSE, \
                     created_at = NOW()",
        )
        .bind(channel_id)
        .bind(from_user)
        .bind(item.to_user)
        .bind(&item.ciphertext)
        .bind(&item.ek_public)
        .execute(state.db.pool())
        .await?;

        // Push immediately to online recipient
        if state.hub.is_online(&item.to_user) {
            state.hub.send_to_user(
                &item.to_user,
                crate::hub::WsOutbound::Message {
                    payload: serde_json::json!({
                        "type": "key_dist",
                        "channel_id": channel_id,
                        "from_user": from_user,
                        "ciphertext": BASE64.encode(&item.ciphertext),
                        "ek_public":  BASE64.encode(&item.ek_public),
                    }),
                },
            );

            sqlx::query(
                "UPDATE sender_key_distributions \
                 SET delivered = TRUE \
                 WHERE channel_id = $1 AND from_user = $2 AND to_user = $3",
            )
            .bind(channel_id)
            .bind(from_user)
            .bind(item.to_user)
            .execute(state.db.pool())
            .await?;
        }
    }

    Ok(())
}

/// Fetch pending sender key distributions for the current user in a channel.
/// Marks them delivered as they are returned.
pub async fn fetch_key_distributions(
    user_id: Uuid,
    channel_id: Uuid,
    state: &AppState,
) -> AppResult<Vec<KeyDistRecord>> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, user_id, server_id).await?;

    let rows = sqlx::query(
        "UPDATE sender_key_distributions \
         SET delivered = TRUE \
         WHERE to_user = $1 AND channel_id = $2 \
         RETURNING from_user, ciphertext, ek_public",
    )
    .bind(user_id)
    .bind(channel_id)
    .fetch_all(state.db.pool())
    .await?;

    rows.iter()
        .map(|r| {
            let ct: Vec<u8> = r.try_get("ciphertext")?;
            let ek: Vec<u8> = r.try_get("ek_public")?;
            Ok(KeyDistRecord {
                from_user: r.try_get("from_user")?,
                ciphertext: BASE64.encode(&ct),
                ek_public: BASE64.encode(&ek),
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)
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
    debug_assert!(sender_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    let member_rows = sqlx::query(
        "SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2",
    )
    .bind(server_id)
    .bind(MAX_FANOUT_MEMBERS as i64)
    .fetch_all(state.db.pool())
    .await?;

    let ws_payload = serde_json::json!({
        "type": "channel",
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
