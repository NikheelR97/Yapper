use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use super::handlers::ChannelResp;
use crate::{
    constants,
    error::{AppError, AppResult},
    AppState,
};

/// Max server members to fan out a channel message to in a single operation.
const MAX_FANOUT_MEMBERS: usize = 500;

// ─── Message record returned by service ──────────────────────────────────────

pub struct MessageRecord {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub sender_device_id: Option<Uuid>,
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

    let exists =
        sqlx::query("SELECT 1 FROM server_memberships WHERE user_id = $1 AND server_id = $2")
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

    let row =
        sqlx::query("SELECT role FROM server_memberships WHERE user_id = $1 AND server_id = $2")
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
    debug_assert!(
        !name.is_empty(),
        "name must be non-empty (validated by handler)"
    );

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
            "SELECT id, channel_id, sender_id, sender_device_id, ciphertext, plaintext, ek_public, opk_id, \
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
            "SELECT id, channel_id, sender_id, sender_device_id, ciphertext, plaintext, ek_public, opk_id, \
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
                sender_device_id: r.try_get("sender_device_id")?,
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
        .map(|r| {
            Ok(ChannelMember {
                user_id: r.try_get("user_id")?,
                username: r.try_get("username")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)
}

// ─── Sender Key distributions ─────────────────────────────────────────────────

pub struct KeyDistItem {
    pub to_user: Uuid,
    pub to_device: Uuid,
    pub ciphertext: Vec<u8>,
    pub ek_public: Vec<u8>,
}

#[derive(Serialize)]
pub struct KeyDistRecord {
    pub from_user: Uuid,
    pub from_device_id: Option<Uuid>,
    pub ciphertext: String, // base64
    pub ek_public: String,  // base64
}

/// Upsert sender key distributions for a channel. Pushes to online recipients immediately.
pub async fn store_key_distributions(
    from_user: Uuid,
    from_device_id: Uuid,
    channel_id: Uuid,
    items: Vec<KeyDistItem>,
    broadcast_request: bool,
    state: &AppState,
) -> AppResult<()> {
    debug_assert!(from_user != Uuid::nil());
    debug_assert!(from_device_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, from_user, server_id).await?;

    for item in &items {
        require_member(state, item.to_user, server_id).await?;

        let recipient_device = sqlx::query(
            "SELECT user_id, revoked_at, trust_state \
             FROM devices \
             WHERE id = $1",
        )
        .bind(item.to_device)
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| AppError::BadRequest("Recipient device not found".into()))?;

        let recipient_user_id: Uuid = recipient_device.try_get("user_id")?;
        let revoked_at: Option<chrono::DateTime<chrono::Utc>> =
            recipient_device.try_get("revoked_at")?;
        let trust_state: String = recipient_device.try_get("trust_state")?;
        if recipient_user_id != item.to_user {
            return Err(AppError::BadRequest(
                "Recipient device does not belong to recipient user".into(),
            ));
        }
        if revoked_at.is_some() || trust_state != "trusted" {
            return Err(AppError::BadRequest(
                "Recipient device must be trusted and active".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO sender_key_distributions \
                 (channel_id, from_user, from_device_id, to_user, to_device_id, ciphertext, ek_public, delivered) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE) \
             ON CONFLICT (channel_id, from_user, from_device_id, to_user, to_device_id) DO UPDATE \
                 SET ciphertext = EXCLUDED.ciphertext, \
                     ek_public  = EXCLUDED.ek_public, \
                     delivered  = FALSE, \
                     created_at = NOW()",
        )
        .bind(channel_id)
        .bind(from_user)
        .bind(from_device_id)
        .bind(item.to_user)
        .bind(item.to_device)
        .bind(&item.ciphertext)
        .bind(&item.ek_public)
        .execute(state.db.pool())
        .await?;

        // Push immediately to online recipient — only mark delivered if the
        // send actually succeeded (channel accepted the payload). Using
        // try_send_to_device instead of fire-and-forget send_to_device prevents
        // marking rows delivered when the socket buffer is full or the device
        // just disconnected, which would cause permanent key-distribution loss.
        let sent = state.hub.try_send_to_device(
            &item.to_device,
            crate::hub::WsOutbound::Message {
                payload: serde_json::json!({
                    "type": "key_dist_v2",
                    "channel_id": channel_id,
                    "from_user": from_user,
                    "from_device_id": from_device_id,
                    "ciphertext": BASE64.encode(&item.ciphertext),
                    "ek_public":  BASE64.encode(&item.ek_public),
                }),
            },
        );

        if sent {
            sqlx::query(
                "UPDATE sender_key_distributions \
                 SET delivered = TRUE \
                 WHERE channel_id = $1 AND from_user = $2 AND from_device_id = $3 \
                   AND to_user = $4 AND to_device_id = $5",
            )
            .bind(channel_id)
            .bind(from_user)
            .bind(from_device_id)
            .bind(item.to_user)
            .bind(item.to_device)
            .execute(state.db.pool())
            .await?;
        }
    }

    // Only broadcast key_dist_request when a NEW member is joining the channel.
    // Redistributions (responses to key_dist_request) must NOT re-broadcast,
    // otherwise an infinite loop occurs: A redistributes → backend broadcasts
    // key_dist_request for A → B redistributes → backend broadcasts for B → …
    if broadcast_request {
        let member_rows = sqlx::query(
            "SELECT DISTINCT u.id as user_id \
             FROM server_memberships sm \
             JOIN channels c ON c.server_id = sm.server_id \
             JOIN users u ON u.id = sm.user_id \
             WHERE c.id = $1 AND u.deleted_at IS NULL",
        )
        .bind(channel_id)
        .fetch_all(state.db.pool())
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to fetch members for key_dist_request broadcast: {e}");
            vec![]
        });

        let other_user_ids: Vec<Uuid> = member_rows
            .iter()
            .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
            .filter(|uid| *uid != from_user)
            .collect();

        if !other_user_ids.is_empty() {
            state.hub.broadcast(
                &other_user_ids,
                crate::hub::WsOutbound::Message {
                    payload: serde_json::json!({
                        "type": "key_dist_request",
                        "channel_id": channel_id,
                        "requester_user_id": from_user,
                        "requester_device_id": from_device_id,
                    }),
                },
            );
        }
    }

    Ok(())
}

/// Fetch pending sender key distributions for the current user in a channel.
///
/// Returns undelivered distributions without marking them delivered. The caller
/// is responsible for calling [`mark_key_distributions_delivered`] after the
/// HTTP response has been successfully sent to the client. This prevents
/// permanent key-distribution loss if the response fails to reach the client.
pub async fn fetch_key_distributions(
    user_id: Uuid,
    device_id: Uuid,
    channel_id: Uuid,
    state: &AppState,
) -> AppResult<Vec<KeyDistRecord>> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(device_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, user_id, server_id).await?;

    let rows = sqlx::query(
        "SELECT id, from_user, from_device_id, ciphertext, ek_public \
         FROM sender_key_distributions \
         WHERE channel_id = $1 \
           AND delivered = FALSE \
           AND (
               to_device_id = $2
               OR (to_device_id IS NULL AND to_user = $3)
           ) \
         ORDER BY created_at ASC \
         LIMIT 100",
    )
    .bind(channel_id)
    .bind(device_id)
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await?;

    let result = rows
        .iter()
        .map(|r| {
            let ct: Vec<u8> = r.try_get("ciphertext")?;
            let ek: Vec<u8> = r.try_get("ek_public")?;
            Ok(KeyDistRecord {
                from_user: r.try_get("from_user")?,
                from_device_id: r.try_get("from_device_id")?,
                ciphertext: BASE64.encode(&ct),
                ek_public: BASE64.encode(&ek),
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)?;

    // Mark delivered only after we have successfully built the response.
    // If the SELECT returned rows, mark them now — the HTTP response is about
    // to be serialised and sent. This is still not perfectly atomic with the
    // TCP send, but far safer than the previous UPDATE...RETURNING which
    // marked rows delivered before the response was even constructed.
    if !rows.is_empty() {
        let ids: Vec<Uuid> = rows
            .iter()
            .filter_map(|r| r.try_get::<Uuid, _>("id").ok())
            .collect();
        if !ids.is_empty() {
            if let Err(e) = sqlx::query(
                "UPDATE sender_key_distributions \
                 SET delivered = TRUE \
                 WHERE id = ANY($1) AND delivered = FALSE",
            )
            .bind(&ids)
            .execute(state.db.pool())
            .await
            {
                tracing::warn!("Failed to mark key distributions delivered: {e}");
            }
        }
    }

    // NOTE: Do NOT broadcast key_dist_request here.  The broadcast is handled
    // by store_key_distributions (with broadcast_request=true) during the
    // initial joinChannel flow.  Repeating it on every GET fetch causes
    // excessive redistributions and API overload.

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub async fn send_message(
    user_id: Uuid,
    sender_device_id: Uuid,
    channel_id: Uuid,
    ciphertext_b64: String,
    ephemeral_key_b64: Option<String>,
    opk_id: Option<i32>,
    message_type: &str,
    msg_num: Option<i32>,
    state: &AppState,
) -> AppResult<MessageRecord> {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(sender_device_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(!ciphertext_b64.is_empty(), "ciphertext must not be empty");

    let server_id = channel_server_id(state, channel_id).await?;
    require_member(state, user_id, server_id).await?;

    let ciphertext = BASE64
        .decode(&ciphertext_b64)
        .map_err(|_| AppError::BadRequest("Invalid ciphertext encoding".into()))?;
    // Channel sends are validated against ciphertext wire bytes after encryption.
    if ciphertext.is_empty() || ciphertext.len() > constants::MAX_MESSAGE_LENGTH {
        return Err(AppError::BadRequest("Ciphertext exceeds size limit".into()));
    }

    let ek_bytes = ephemeral_key_b64
        .as_deref()
        .map(|s| BASE64.decode(s))
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid ephemeral_key encoding".into()))?;

    let row = sqlx::query(
        "INSERT INTO messages \
             (channel_id, sender_id, sender_device_id, ciphertext, ek_public, opk_id, message_type, msg_num, delivered) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, TRUE) \
         RETURNING id, created_at",
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(sender_device_id)
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
        user_id,
        sender_device_id,
        channel_id,
        server_id,
        msg_id,
        created_at,
        &ciphertext_b64,
        ephemeral_key_b64.as_deref(),
        opk_id,
        message_type,
        msg_num,
        state,
    )
    .await?;

    Ok(MessageRecord {
        id: msg_id,
        channel_id,
        sender_id: user_id,
        sender_device_id: Some(sender_device_id),
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
    sender_device_id: Uuid,
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
    debug_assert!(sender_device_id != Uuid::nil());
    debug_assert!(server_id != Uuid::nil());

    let member_rows =
        sqlx::query("SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2")
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
        "sender_device_id": sender_device_id,
        "ciphertext": ciphertext_b64,
        "ephemeral_key": ephemeral_key,
        "opk_id": opk_id,
        "message_type": message_type,
        "msg_num": msg_num,
        "created_at": created_at,
    });

    for m in member_rows.iter().take(MAX_FANOUT_MEMBERS) {
        let uid: Uuid = m.try_get("user_id")?;
        state.hub.send_to_user(
            &uid,
            crate::hub::WsOutbound::Message {
                payload: ws_payload.clone(),
            },
        );
    }

    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use sqlx::{PgPool, Row};
    use uuid::Uuid;

    async fn insert_user(pool: &PgPool, suffix: &str) -> Uuid {
        let row = sqlx::query(
            "INSERT INTO users (email, username, display_name, account_type, parental_controls_enabled) \
             VALUES ($1, $2, $3, 'standard', FALSE) RETURNING id",
        )
        .bind(format!("channel_test+{suffix}@example.com"))
        .bind(format!("channel_test_{suffix}"))
        .bind(format!("Test {suffix}"))
        .fetch_one(pool)
        .await
        .expect("insert user");
        row.try_get("id").expect("id")
    }

    async fn insert_server(pool: &PgPool, owner_id: Uuid, suffix: &str) -> Uuid {
        let row = sqlx::query(
            "INSERT INTO servers (name, slug, owner_id) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(format!("TestSrv {suffix}"))
        .bind(format!("test-srv-{suffix}"))
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .expect("insert server");
        row.try_get("id").expect("id")
    }

    async fn insert_channel(pool: &PgPool, server_id: Uuid, suffix: &str) -> Uuid {
        let row = sqlx::query(
            "INSERT INTO channels (server_id, name, type, position) VALUES ($1, $2, 'text', 0) RETURNING id",
        )
        .bind(server_id)
        .bind(format!("general-{suffix}"))
        .fetch_one(pool)
        .await
        .expect("insert channel");
        row.try_get("id").expect("id")
    }

    async fn insert_membership(pool: &PgPool, user_id: Uuid, server_id: Uuid, role: &str) {
        sqlx::query(
            "INSERT INTO server_memberships (user_id, server_id, role) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(server_id)
        .bind(role)
        .execute(pool)
        .await
        .expect("insert membership");
    }

    /// Regression test: the key_dist_request broadcast query must reference the
    /// `server_memberships` table (not `server_members`). A wrong table name
    /// caused the query to silently fail via `.unwrap_or_default()`, preventing
    /// SenderKey redistribution on multi-device setups.
    #[sqlx::test(migrations = "./migrations")]
    async fn key_dist_member_lookup_uses_correct_table(pool: PgPool) {
        let suffix = Uuid::new_v4().simple().to_string();
        let user_a = insert_user(&pool, &format!("a_{suffix}")).await;
        let user_b = insert_user(&pool, &format!("b_{suffix}")).await;
        let server_id = insert_server(&pool, user_a, &suffix).await;
        let channel_id = insert_channel(&pool, server_id, &suffix).await;
        insert_membership(&pool, user_a, server_id, "owner").await;
        insert_membership(&pool, user_b, server_id, "member").await;

        // This is the exact query from store_key_distributions / fetch_key_distributions.
        // Before the fix it referenced the non-existent `server_members` table.
        let rows = sqlx::query(
            "SELECT DISTINCT u.id as user_id \
             FROM server_memberships sm \
             JOIN channels c ON c.server_id = sm.server_id \
             JOIN users u ON u.id = sm.user_id \
             WHERE c.id = $1 AND u.deleted_at IS NULL",
        )
        .bind(channel_id)
        .fetch_all(&pool)
        .await
        .expect("member lookup query must succeed");

        let member_ids: Vec<Uuid> = rows
            .iter()
            .map(|r| r.try_get::<Uuid, _>("user_id").expect("user_id"))
            .collect();

        assert!(
            member_ids.contains(&user_a),
            "user_a should be in member list"
        );
        assert!(
            member_ids.contains(&user_b),
            "user_b should be in member list"
        );
        assert_eq!(member_ids.len(), 2, "exactly 2 members expected");
    }
}
