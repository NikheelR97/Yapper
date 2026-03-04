use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::service;
use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

// ─── Response types (pub — used by servers router) ───────────────────────────

#[derive(Serialize)]
pub struct ChannelResp {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: String,
    pub position: i32,
}

#[derive(Serialize)]
pub(super) struct MessageResp {
    id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    ciphertext: Option<String>,
    plaintext: Option<String>,
    ephemeral_key: Option<String>,
    opk_id: Option<i32>,
    message_type: String,
    msg_num: Option<i32>,
    created_at: chrono::DateTime<chrono::Utc>,
}

// ─── Request DTOs ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateChannelReq {
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MessagesQuery {
    before: Option<Uuid>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SendMessageReq {
    ciphertext: String,
    ephemeral_key: Option<String>,
    opk_id: Option<i32>,
    message_type: Option<String>,
    msg_num: Option<i32>,
}

// ─── Channel CRUD (pub — called from servers router) ─────────────────────────

pub async fn list_channels(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<ChannelResp>>> {
    let channels = service::list_channels(auth.user_id, server_id, &state).await?;
    Ok(Json(channels))
}

pub async fn create_channel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<CreateChannelReq>,
) -> AppResult<(StatusCode, Json<ChannelResp>)> {
    let name = req.name.trim().to_lowercase();
    if name.is_empty() || name.len() > 50 {
        return Err(AppError::BadRequest(
            "Channel name must be 1–50 characters".into(),
        ));
    }
    let channel_type = req.channel_type.as_deref().unwrap_or("text");
    if !["text", "voice", "announcement"].contains(&channel_type) {
        return Err(AppError::BadRequest("Invalid channel type".into()));
    }
    let channel =
        service::create_channel(auth.user_id, server_id, name, channel_type, &state).await?;
    Ok((StatusCode::CREATED, Json(channel)))
}

// ─── Message handlers ─────────────────────────────────────────────────────────

pub(super) async fn get_messages(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> AppResult<Json<Vec<MessageResp>>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let msgs = service::get_messages(auth.user_id, channel_id, query.before, limit, &state).await?;
    let resp: Vec<MessageResp> = msgs
        .into_iter()
        .map(|m| MessageResp {
            id: m.id,
            channel_id: m.channel_id,
            sender_id: m.sender_id,
            ciphertext: m.ciphertext,
            plaintext: m.plaintext,
            ephemeral_key: m.ephemeral_key,
            opk_id: m.opk_id,
            message_type: m.message_type,
            msg_num: m.msg_num,
            created_at: m.created_at,
        })
        .collect();
    Ok(Json(resp))
}

// ─── Channel members ──────────────────────────────────────────────────────────

pub async fn list_members(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
) -> AppResult<Json<Vec<service::ChannelMember>>> {
    let members = service::list_channel_members(auth.user_id, channel_id, &state).await?;
    Ok(Json(members))
}

// ─── Sender Key distributions ─────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KeyDistItem {
    to_user_id: Uuid,
    /// Base64-encoded ECIES ciphertext.
    ciphertext: String,
    /// Base64-encoded X25519 ephemeral public key.
    ek_public: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PostKeyDistsReq {
    distributions: Vec<KeyDistItem>,
}

pub(super) async fn post_key_dists(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<PostKeyDistsReq>,
) -> AppResult<Json<serde_json::Value>> {
    if req.distributions.is_empty() || req.distributions.len() > 500 {
        return Err(AppError::BadRequest("Provide 1–500 distributions".into()));
    }

    let items = req
        .distributions
        .into_iter()
        .map(|d| {
            let ciphertext = BASE64
                .decode(&d.ciphertext)
                .map_err(|_| AppError::BadRequest("Invalid ciphertext encoding".into()))?;
            let ek_public = BASE64
                .decode(&d.ek_public)
                .map_err(|_| AppError::BadRequest("Invalid ek_public encoding".into()))?;
            if ek_public.len() != 32 {
                return Err(AppError::BadRequest("ek_public must be 32 bytes".into()));
            }
            Ok(service::KeyDistItem {
                to_user: d.to_user_id,
                ciphertext,
                ek_public,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    service::store_key_distributions(auth.user_id, channel_id, items, &state).await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub(super) async fn get_key_dists(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
) -> AppResult<Json<Vec<service::KeyDistRecord>>> {
    let dists = service::fetch_key_distributions(auth.user_id, channel_id, &state).await?;
    Ok(Json(dists))
}

pub(super) async fn send_message(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageReq>,
) -> AppResult<(StatusCode, Json<MessageResp>)> {
    let message_type = req.message_type.as_deref().unwrap_or("text");
    if !["text", "yap", "clip"].contains(&message_type) {
        return Err(AppError::BadRequest("Invalid message_type".into()));
    }
    let msg = service::send_message(
        auth.user_id,
        channel_id,
        req.ciphertext,
        req.ephemeral_key,
        req.opk_id,
        message_type,
        req.msg_num,
        &state,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(MessageResp {
            id: msg.id,
            channel_id: msg.channel_id,
            sender_id: msg.sender_id,
            ciphertext: msg.ciphertext,
            plaintext: msg.plaintext,
            ephemeral_key: msg.ephemeral_key,
            opk_id: msg.opk_id,
            message_type: msg.message_type,
            msg_num: msg.msg_num,
            created_at: msg.created_at,
        }),
    ))
}
