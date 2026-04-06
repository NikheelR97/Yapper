use axum::{
    extract::{Path, Query, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthDevice,
    error::{AppError, AppResult},
    hub::mark_dm_delivered,
    AppState,
};

use super::service;
use super::types::*;

pub async fn create_or_get_conversation_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(req): Json<CreateConversationReq>,
) -> AppResult<Json<ConversationResp>> {
    auth.require_trusted()?;
    service::create_or_get_conversation_for_user(auth.user_id, req, state).await
}

pub async fn list_conversations_v2(
    auth: AuthDevice,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ConversationListItem>>> {
    auth.require_trusted()?;
    service::list_conversations_for_user(auth.user_id, state).await
}

pub async fn list_messages_v2(
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
        .map(|r| -> Result<MessageRespV2, sqlx::Error> {
            let cipher: Vec<u8> = r.try_get("ciphertext").unwrap_or_default();
            let ek: Option<Vec<u8>> = r.try_get("ek_public").ok().flatten();
            let ratchet_pub: Option<Vec<u8>> = r.try_get("ratchet_pub").ok().flatten();
            Ok(MessageRespV2 {
                id: r.try_get("id")?,
                conversation_id: r.try_get("conversation_id")?,
                sender_id: r.try_get("sender_id")?,
                sender_device_id: r.try_get("sender_device_id")?,
                sender_signal_device_id: r.try_get("sender_signal_device_id")?,
                ciphertext: BASE64.encode(&cipher),
                ephemeral_key: ek.as_ref().map(|k| BASE64.encode(k)),
                opk_id: r.try_get("opk_id").ok().flatten(),
                msg_num: r.try_get("msg_num").unwrap_or(0),
                ratchet_pub: ratchet_pub.as_ref().map(|key| BASE64.encode(key)),
                previous_chain_len: r.try_get("previous_chain_len").ok().flatten(),
                crypto_version: r.try_get("crypto_version").unwrap_or(1),
                created_at: r.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    messages.reverse();
    Ok(Json(messages))
}

pub async fn send_message_v2(
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

    service::validate_dm_participants(conv_id, auth.user_id, &req, &state).await?;
    service::validate_dm_recipient_devices(&req, &state).await?;

    let message_id = Uuid::new_v4();
    let created_at = Utc::now();
    service::store_dm_envelopes(message_id, conv_id, created_at, &auth, &req, &state).await?;

    let delivered = service::fanout_dm_v2(message_id, conv_id, created_at, &auth, &req, &state);
    if let Err(e) = mark_dm_delivered(&delivered, &state).await {
        tracing::warn!("Failed to mark DM v2 delivered: {e}");
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message_id": message_id,
        "created_at": created_at,
    })))
}
