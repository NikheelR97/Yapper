use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use governor::{clock::DefaultClock, state::{InMemoryState, NotKeyed}, RateLimiter};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{num::NonZeroU32, sync::Arc};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;

/// Max frame size: 64KB (Rule 3).
const MAX_WS_FRAME_SIZE: usize = 64 * 1024;
/// Max server members to fan out a channel message to (Rule 2 / Rule 3).
const MAX_FANOUT_MEMBERS: i64 = 500;

/// Per-user WebSocket message rate limiter (5 msg/sec, burst of 20).
type MsgRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Identifies a connected device session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Per-connection sender. Messages sent here are forwarded to the WebSocket.
pub type ConnTx = mpsc::UnboundedSender<WsOutbound>;

/// The hub holds all active connections indexed by UserId -> ConnectionId -> Sender.
/// DashMap gives lock-free concurrent reads (critical for high fan-out).
pub struct Hub {
    /// user_id -> map of connection_id -> sender
    connections: DashMap<Uuid, DashMap<ConnectionId, ConnTx>>,
    /// Per-user message rate limiters — cleaned up on full disconnect.
    msg_limiters: DashMap<Uuid, MsgRateLimiter>,
    /// Typing auto-stop timers keyed by (channel_id, user_id). Aborted on new TypingStart.
    pub typing_timers: DashMap<(Uuid, Uuid), tokio::task::JoinHandle<()>>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            msg_limiters: DashMap::new(),
            typing_timers: DashMap::new(),
        }
    }

    /// Returns true if the user is within their message rate limit (5/sec, burst 20).
    fn check_msg_rate(&self, user_id: &Uuid) -> bool {
        let limiter = self
            .msg_limiters
            .entry(*user_id)
            .or_insert_with(|| {
                let quota = governor::Quota::per_second(NonZeroU32::new(5).unwrap())
                    .allow_burst(NonZeroU32::new(20).unwrap());
                Arc::new(RateLimiter::direct(quota))
            })
            .clone();
        limiter.check().is_ok()
    }

    pub fn register(&self, user_id: Uuid, conn_id: ConnectionId, tx: ConnTx) {
        self.connections
            .entry(user_id)
            .or_default()
            .insert(conn_id, tx);
    }

    pub fn unregister(&self, user_id: &Uuid, conn_id: &ConnectionId) {
        if let Some(user_conns) = self.connections.get(user_id) {
            user_conns.remove(conn_id);
            if user_conns.is_empty() {
                drop(user_conns);
                self.connections.remove(user_id);
                self.msg_limiters.remove(user_id);
            }
        }
    }

    pub fn is_online(&self, user_id: &Uuid) -> bool {
        self.connections
            .get(user_id)
            .map(|m| !m.is_empty())
            .unwrap_or(false)
    }

    /// Send a message to all connections of a specific user.
    pub fn send_to_user(&self, user_id: &Uuid, msg: WsOutbound) {
        if let Some(user_conns) = self.connections.get(user_id) {
            for entry in user_conns.iter() {
                let _ = entry.value().send(msg.clone());
            }
        }
    }

    /// Fan out a message to multiple users (e.g. all members of a channel).
    pub fn broadcast(&self, user_ids: &[Uuid], msg: WsOutbound) {
        for user_id in user_ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
            self.send_to_user(user_id, msg.clone());
        }
    }
}

/// Messages sent FROM client TO server over WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsInbound {
    Auth { token: String },
    Reauth { token: String },
    SendDm {
        conversation_id: Uuid,
        ciphertext: String,
        ephemeral_key: Option<String>,
        opk_id: Option<i32>,
        msg_num: u32,
    },
    SendChannel {
        channel_id: Uuid,
        ciphertext: String,
        message_type: Option<String>,
        msg_num: Option<i32>,
    },
    TypingStart { channel_id: Uuid },
    Read { message_id: Uuid, channel_id: Uuid },
    Ping,
}

/// Messages sent FROM server TO client over WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsOutbound {
    Ready { user_id: Uuid },
    Message { payload: serde_json::Value },
    Typing { channel_id: Uuid, user_id: Uuid },
    TypingStop { channel_id: Uuid, user_id: Uuid },
    ReadReceipt { message_id: Uuid, user_id: Uuid },
    ReAuthRequired,
    CanvasUpdate { payload: serde_json::Value },
    ParentNotification { payload: serde_json::Value },
    Error { code: u16, message: String },
    Pong,
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn send_ws_error(tx: &ConnTx, code: u16, message: &str) {
    let _ = tx.send(WsOutbound::Error { code, message: message.to_string() });
}

fn check_rate_limit(state: &AppState, user_id: &Uuid, tx: &ConnTx) -> bool {
    if !state.hub.check_msg_rate(user_id) {
        send_ws_error(tx, 4029, "Message rate limit exceeded");
        return false;
    }
    true
}

/// WebSocket upgrade handler — attached to GET /ws
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

// ─── Socket lifecycle ────────────────────────────────────────────────────────

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let conn_id = ConnectionId::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsOutbound>();

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = match serde_json::to_string(&msg) {
                Ok(t) => t,
                Err(e) => { tracing::error!("WS serialize error: {e}"); continue; }
            };
            if sender.send(Message::Text(text)).await.is_err() { break; }
        }
    });

    let user_id = match wait_for_auth(&mut receiver, &state).await {
        Some(id) => id,
        None => { send_task.abort(); return; }
    };

    state.hub.register(user_id, conn_id.clone(), tx.clone());
    let _ = tx.send(WsOutbound::Ready { user_id });
    deliver_offline_messages(&user_id, &state, &tx).await;
    deliver_pending_key_dists(&user_id, &state, &tx).await;

    run_receive_loop(&mut receiver, &mut send_task, user_id, &state, &tx).await;

    state.hub.unregister(&user_id, &conn_id);
    update_last_seen(user_id, &state);
}

async fn run_receive_loop(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    send_task: &mut tokio::task::JoinHandle<()>,
    user_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_inbound(text, user_id, state, tx).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(_))) => { let _ = tx.send(WsOutbound::Pong); }
                    _ => {}
                }
            }
            _ = &mut *send_task => break,
        }
    }
}

fn update_last_seen(user_id: Uuid, state: &AppState) {
    let pool = state.db.pool().clone();
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await;
    });
}

// ─── Auth ────────────────────────────────────────────────────────────────────

async fn wait_for_auth(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    state: &AppState,
) -> Option<Uuid> {
    let timeout = tokio::time::Duration::from_secs(10);
    let result = tokio::time::timeout(timeout, receiver.next()).await;

    let frame = match result {
        Ok(Some(Ok(Message::Text(text)))) => text,
        _ => return None,
    };

    let msg: WsInbound = serde_json::from_str(&frame).ok()?;
    match msg {
        WsInbound::Auth { token } => validate_ws_token(&token, state).await,
        _ => None,
    }
}

async fn validate_ws_token(token: &str, state: &AppState) -> Option<Uuid> {
    crate::auth::validate_ws_token(token, &state.jwt_keys)
}

// ─── Offline delivery ────────────────────────────────────────────────────────

async fn deliver_offline_messages(user_id: &Uuid, state: &AppState, tx: &ConnTx) {
    debug_assert!(*user_id != Uuid::nil());

    let rows = sqlx::query(
        "SELECT m.id, m.conversation_id, m.sender_id, m.ciphertext, m.ek_public, m.opk_id \
         FROM messages m \
         JOIN dm_participants dp ON m.conversation_id = dp.conversation_id AND dp.user_id = $1 \
         WHERE m.delivered = FALSE AND m.sender_id != $1 \
           AND m.ciphertext IS NOT NULL AND m.deleted_at IS NULL \
         ORDER BY m.created_at ASC LIMIT 100",
    )
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await;

    let Ok(rows) = rows else { return };
    if rows.is_empty() { return; }

    let mut delivered_ids: Vec<Uuid> = Vec::with_capacity(rows.len());

    for row in &rows {
        let Ok(msg_id) = row.try_get::<Uuid, _>("id") else { continue };
        let Ok(conv_id) = row.try_get::<Uuid, _>("conversation_id") else { continue };
        let Ok(sender_id) = row.try_get::<Uuid, _>("sender_id") else { continue };
        let Ok(cipher) = row.try_get::<Vec<u8>, _>("ciphertext") else { continue };
        let ek: Option<Vec<u8>> = row.try_get("ek_public").ok().flatten();
        let opk_id: Option<i32> = row.try_get("opk_id").ok().flatten();

        let payload = serde_json::json!({
            "type": "dm", "id": msg_id, "conversation_id": conv_id,
            "sender_id": sender_id, "ciphertext": BASE64.encode(&cipher),
            "ephemeral_key": ek.as_ref().map(|k| BASE64.encode(k)), "opk_id": opk_id,
        });

        if tx.send(WsOutbound::Message { payload }).is_ok() {
            delivered_ids.push(msg_id);
        }
    }

    if !delivered_ids.is_empty() {
        let _ = sqlx::query("UPDATE messages SET delivered = TRUE WHERE id = ANY($1)")
            .bind(&delivered_ids)
            .execute(state.db.pool())
            .await;
    }
}

// ─── Offline sender key distributions ────────────────────────────────────────

async fn deliver_pending_key_dists(user_id: &Uuid, state: &AppState, tx: &ConnTx) {
    debug_assert!(*user_id != Uuid::nil());

    let rows = sqlx::query(
        "UPDATE sender_key_distributions \
         SET delivered = TRUE \
         WHERE to_user = $1 AND delivered = FALSE \
         RETURNING channel_id, from_user, ciphertext, ek_public",
    )
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await;

    let Ok(rows) = rows else { return };

    for row in &rows {
        let Ok(channel_id) = row.try_get::<uuid::Uuid, _>("channel_id") else { continue };
        let Ok(from_user) = row.try_get::<uuid::Uuid, _>("from_user") else { continue };
        let Ok(ct) = row.try_get::<Vec<u8>, _>("ciphertext") else { continue };
        let Ok(ek) = row.try_get::<Vec<u8>, _>("ek_public") else { continue };

        let payload = serde_json::json!({
            "type": "key_dist",
            "channel_id": channel_id,
            "from_user": from_user,
            "ciphertext": BASE64.encode(&ct),
            "ek_public":  BASE64.encode(&ek),
        });
        let _ = tx.send(WsOutbound::Message { payload });
    }
}

// ─── Inbound dispatch ────────────────────────────────────────────────────────

async fn handle_inbound(text: String, user_id: Uuid, state: &AppState, tx: &ConnTx) {
    if text.len() > MAX_WS_FRAME_SIZE {
        send_ws_error(tx, 4003, "Frame too large");
        return;
    }

    let msg: WsInbound = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(_) => { send_ws_error(tx, 4000, "Invalid message format"); return; }
    };

    match msg {
        WsInbound::Ping => { let _ = tx.send(WsOutbound::Pong); }
        WsInbound::SendDm { conversation_id, ciphertext, ephemeral_key, opk_id, msg_num } => {
            if check_rate_limit(state, &user_id, tx) {
                handle_send_dm(conversation_id, ciphertext, ephemeral_key, opk_id, msg_num, user_id, state, tx).await;
            }
        }
        WsInbound::SendChannel { channel_id, ciphertext, message_type, msg_num } => {
            if check_rate_limit(state, &user_id, tx) {
                handle_send_channel(channel_id, ciphertext, message_type, msg_num, user_id, state, tx).await;
            }
        }
        WsInbound::TypingStart { channel_id } => {
            handle_typing_start(channel_id, user_id, state).await;
        }
        WsInbound::Read { message_id, channel_id } => {
            handle_mark_read(message_id, channel_id, user_id, state).await;
        }
        WsInbound::Reauth { token } => {
            if validate_ws_token(&token, state).await.is_none() {
                send_ws_error(tx, 4001, "Invalid token");
            }
        }
        WsInbound::Auth { .. } => {}
    }
}

// ─── Send DM ─────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_send_dm(
    conversation_id: Uuid,
    ciphertext: String,
    ephemeral_key: Option<String>,
    opk_id: Option<i32>,
    msg_num: u32,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
    debug_assert!(sender_id != Uuid::nil());

    let cipher_bytes = match BASE64.decode(&ciphertext) {
        Ok(b) => b,
        Err(_) => { send_ws_error(tx, 4005, "Invalid ciphertext encoding"); return; }
    };
    let ek_bytes = ephemeral_key.as_ref().and_then(|k| BASE64.decode(k).ok());

    let recipient_id = match resolve_dm_recipient(conversation_id, sender_id, state, tx).await {
        Some(id) => id,
        None => return,
    };

    store_and_route_dm(
        conversation_id, &cipher_bytes, ek_bytes.as_deref(), opk_id, msg_num,
        &ciphertext, &ephemeral_key, sender_id, recipient_id, state, tx,
    ).await;
}

async fn resolve_dm_recipient(
    conversation_id: Uuid,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) -> Option<Uuid> {
    let is_participant = sqlx::query(
        "SELECT 1 FROM dm_participants WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .fetch_optional(state.db.pool())
    .await
    .ok()
    .flatten()
    .is_some();

    if !is_participant {
        send_ws_error(tx, 4006, "Not a participant in this conversation");
        return None;
    }

    let recipient_row = sqlx::query(
        "SELECT user_id FROM dm_participants WHERE conversation_id = $1 AND user_id != $2 LIMIT 1",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .fetch_optional(state.db.pool())
    .await;

    match recipient_row {
        Ok(Some(row)) => row.try_get("user_id").ok(),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
async fn store_and_route_dm(
    conversation_id: Uuid,
    cipher_bytes: &[u8],
    ek_bytes: Option<&[u8]>,
    opk_id: Option<i32>,
    msg_num: u32,
    ciphertext_b64: &str,
    ephemeral_key: &Option<String>,
    sender_id: Uuid,
    recipient_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
    let recipient_online = state.hub.is_online(&recipient_id);
    let msg_id = Uuid::new_v4();

    let store_result = sqlx::query(
        "INSERT INTO messages (id, conversation_id, sender_id, ciphertext, ek_public, opk_id, delivered) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(msg_id)
    .bind(conversation_id)
    .bind(sender_id)
    .bind(cipher_bytes)
    .bind(ek_bytes)
    .bind(opk_id)
    .bind(recipient_online)
    .execute(state.db.pool())
    .await;

    if let Err(e) = store_result {
        tracing::error!("Failed to store DM: {e}");
        send_ws_error(tx, 4007, "Failed to store message");
        return;
    }

    let payload = serde_json::json!({
        "type": "dm", "id": msg_id, "conversation_id": conversation_id,
        "sender_id": sender_id, "ciphertext": ciphertext_b64,
        "ephemeral_key": ephemeral_key, "opk_id": opk_id, "msg_num": msg_num,
    });
    state.hub.send_to_user(&recipient_id, WsOutbound::Message { payload });
}

// ─── Send Channel ────────────────────────────────────────────────────────────

async fn handle_send_channel(
    channel_id: Uuid,
    ciphertext: String,
    message_type: Option<String>,
    msg_num: Option<i32>,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
    debug_assert!(sender_id != Uuid::nil());

    let cipher_bytes = match BASE64.decode(&ciphertext) {
        Ok(b) => b,
        Err(_) => { send_ws_error(tx, 4005, "Invalid ciphertext encoding"); return; }
    };

    let server_id = match resolve_channel_membership(channel_id, sender_id, state, tx).await {
        Some(id) => id,
        None => return,
    };

    let msg_type = message_type.as_deref().unwrap_or("text");
    store_and_fanout_channel(
        channel_id, server_id, &cipher_bytes, msg_type, msg_num,
        &ciphertext, sender_id, state, tx,
    ).await;
}

async fn resolve_channel_membership(
    channel_id: Uuid,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) -> Option<Uuid> {
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(state.db.pool())
        .await
        .ok()
        .flatten();

    let server_id: Uuid = match server_row.and_then(|r| r.try_get("server_id").ok()) {
        Some(id) => id,
        None => { send_ws_error(tx, 4006, "Channel not found"); return None; }
    };

    let is_member = sqlx::query(
        "SELECT 1 FROM server_memberships WHERE user_id = $1 AND server_id = $2",
    )
    .bind(sender_id)
    .bind(server_id)
    .fetch_optional(state.db.pool())
    .await
    .ok()
    .flatten()
    .is_some();

    if !is_member {
        send_ws_error(tx, 4006, "Not a member of this server");
        return None;
    }

    Some(server_id)
}

#[allow(clippy::too_many_arguments)]
async fn store_and_fanout_channel(
    channel_id: Uuid,
    server_id: Uuid,
    cipher_bytes: &[u8],
    msg_type: &str,
    msg_num: Option<i32>,
    ciphertext_b64: &str,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
    let msg_id = Uuid::new_v4();

    let store_result = sqlx::query(
        "INSERT INTO messages (id, channel_id, sender_id, ciphertext, message_type, msg_num, delivered) \
         VALUES ($1, $2, $3, $4, $5, $6, TRUE)",
    )
    .bind(msg_id)
    .bind(channel_id)
    .bind(sender_id)
    .bind(cipher_bytes)
    .bind(msg_type)
    .bind(msg_num)
    .execute(state.db.pool())
    .await;

    if let Err(e) = store_result {
        tracing::error!("Failed to store channel message: {e}");
        send_ws_error(tx, 4007, "Failed to store message");
        return;
    }

    let member_rows = match sqlx::query(
        "SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2",
    )
    .bind(server_id)
    .bind(MAX_FANOUT_MEMBERS)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(rows) => rows,
        Err(e) => { tracing::error!("Failed to fetch members for fanout: {e}"); return; }
    };

    let payload = serde_json::json!({
        "type": "channel", "id": msg_id, "channel_id": channel_id,
        "server_id": server_id, "sender_id": sender_id,
        "ciphertext": ciphertext_b64, "message_type": msg_type, "msg_num": msg_num,
    });

    for m in member_rows.iter().take(MAX_FANOUT_MEMBERS as usize) {
        if let Ok(uid) = m.try_get::<Uuid, _>("user_id") {
            if uid != sender_id {
                state.hub.send_to_user(&uid, WsOutbound::Message { payload: payload.clone() });
            }
        }
    }
}

// ─── Typing indicators ────────────────────────────────────────────────────────

/// Resolves member UUIDs for a channel (bounded to MAX_FANOUT_MEMBERS).
async fn fetch_channel_member_ids(channel_id: Uuid, state: &AppState) -> Option<Vec<Uuid>> {
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(MAX_FANOUT_MEMBERS >= 100);

    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(state.db.pool())
        .await
        .ok()
        .flatten()?;
    let server_id: Uuid = server_row.try_get("server_id").ok()?;

    let rows = sqlx::query(
        "SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2",
    )
    .bind(server_id)
    .bind(MAX_FANOUT_MEMBERS)
    .fetch_all(state.db.pool())
    .await
    .ok()?;

    Some(rows.iter().filter_map(|r| r.try_get::<Uuid, _>("user_id").ok()).collect())
}

/// Fan out a TypingStart event and schedule an auto-stop after 5 seconds of silence.
async fn handle_typing_start(channel_id: Uuid, user_id: Uuid, state: &AppState) {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());

    let Some(member_ids) = fetch_channel_member_ids(channel_id, state).await else { return };
    let key = (channel_id, user_id);

    // Abort any existing timer to reset the 5-second window
    if let Some((_, old)) = state.hub.typing_timers.remove(&key) {
        old.abort();
    }

    for uid in member_ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
        if *uid != user_id {
            state.hub.send_to_user(uid, WsOutbound::Typing { channel_id, user_id });
        }
    }

    let hub = Arc::clone(&state.hub);
    let ids = member_ids;
    let handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        for uid in ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
            if *uid != user_id {
                hub.send_to_user(uid, WsOutbound::TypingStop { channel_id, user_id });
            }
        }
        hub.typing_timers.remove(&key);
    });
    state.hub.typing_timers.insert(key, handle);
}

// ─── Read receipts ────────────────────────────────────────────────────────────

/// Upsert a read receipt and fan out ReadReceipt to all channel members.
async fn handle_mark_read(message_id: Uuid, channel_id: Uuid, user_id: Uuid, state: &AppState) {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(message_id != Uuid::nil());

    let ok = sqlx::query(
        "INSERT INTO message_read_receipts (message_id, user_id) \
         VALUES ($1, $2) ON CONFLICT (message_id, user_id) DO NOTHING",
    )
    .bind(message_id)
    .bind(user_id)
    .execute(state.db.pool())
    .await;

    if ok.is_err() {
        return;
    }

    let Some(member_ids) = fetch_channel_member_ids(channel_id, state).await else { return };
    for uid in member_ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
        state.hub.send_to_user(uid, WsOutbound::ReadReceipt { message_id, user_id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_register_unregister() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        let conn_id = ConnectionId::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        assert!(!hub.is_online(&user_id));
        hub.register(user_id, conn_id.clone(), tx);
        assert!(hub.is_online(&user_id));
        hub.unregister(&user_id, &conn_id);
        assert!(!hub.is_online(&user_id));
    }

    #[test]
    fn test_hub_send_to_offline_user_is_noop() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        hub.send_to_user(&user_id, WsOutbound::Pong);
    }
}
