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

/// The hub holds all active connections indexed by UserId → ConnectionId → Sender.
/// DashMap gives lock-free concurrent reads (critical for high fan-out).
pub struct Hub {
    /// user_id → map of connection_id → sender
    connections: DashMap<Uuid, DashMap<ConnectionId, ConnTx>>,
    /// Per-user message rate limiters — cleaned up on full disconnect.
    msg_limiters: DashMap<Uuid, MsgRateLimiter>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            msg_limiters: DashMap::new(),
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
        for user_id in user_ids {
            self.send_to_user(user_id, msg.clone());
        }
    }
}

/// Messages sent FROM client TO server over WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsInbound {
    /// First message after connect — authenticate the connection.
    Auth { token: String },
    /// Re-authenticate before token expiry (server sends re_auth_required first).
    Reauth { token: String },
    /// Send an E2EE direct message.
    SendDm {
        conversation_id: Uuid,
        /// Base64-encoded AES-256-GCM ciphertext (IV prepended).
        ciphertext: String,
        /// Base64-encoded X25519 ephemeral key — present only on first message (X3DH).
        ephemeral_key: Option<String>,
        /// OPK id used in X3DH — present only on first message.
        opk_id: Option<i32>,
        /// Monotonic message number within the sender's ratchet chain.
        msg_num: u32,
    },
    /// Typing indicator — client sends every 3 seconds while typing.
    TypingStart { channel_id: Uuid },
    /// Read receipt — sent when message enters viewport.
    Read { message_id: Uuid, channel_id: Uuid },
    /// Ping keepalive.
    Ping,
}

/// Messages sent FROM server TO client over WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsOutbound {
    /// Sent after successful auth.
    Ready { user_id: Uuid },
    /// New message delivered.
    Message { payload: serde_json::Value },
    /// Typing indicator received.
    Typing { channel_id: Uuid, user_id: Uuid },
    /// Typing stopped.
    TypingStop { channel_id: Uuid, user_id: Uuid },
    /// Read receipt received.
    ReadReceipt { message_id: Uuid, user_id: Uuid },
    /// Tell client to refresh its JWT before it expires.
    ReAuthRequired,
    /// Canvas update (music state, poll vote, etc.).
    CanvasUpdate { payload: serde_json::Value },
    /// Parental notification for parent accounts.
    ParentNotification { payload: serde_json::Value },
    /// Error frame.
    Error { code: u16, message: String },
    /// Pong keepalive.
    Pong,
}

/// WebSocket upgrade handler — attached to GET /ws
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let conn_id = ConnectionId::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsOutbound>();

    // Spawn task: forward outbound channel messages → WebSocket frames
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = match serde_json::to_string(&msg) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("WS serialize error: {e}");
                    continue;
                }
            };
            if sender.send(Message::Text(text)).await.is_err() {
                break; // Client disconnected
            }
        }
    });

    // Auth gate — first frame must be { "type": "auth", "token": "..." }
    // Token is NOT passed in the query string (avoids server/proxy log leakage).
    let user_id = match wait_for_auth(&mut receiver, &state).await {
        Some(id) => id,
        None => {
            send_task.abort();
            return;
        }
    };

    state.hub.register(user_id, conn_id.clone(), tx.clone());
    let _ = tx.send(WsOutbound::Ready { user_id });

    // Deliver any offline messages queued while client was away
    deliver_offline_messages(&user_id, &state, &tx).await;

    // Main receive loop
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_inbound(text, user_id, &state, &tx).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = tx.send(WsOutbound::Pong);
                        drop(data);
                    }
                    _ => {} // Binary frames / Pong ignored
                }
            }
            _ = &mut send_task => break, // Send task exited (client gone)
        }
    }

    state.hub.unregister(&user_id, &conn_id);
    // Update last_seen_at on disconnect (fire-and-forget)
    let pool = state.db.pool().clone();
    let uid = user_id;
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
            .bind(uid)
            .execute(&pool)
            .await;
    });
}

async fn wait_for_auth(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    state: &AppState,
) -> Option<Uuid> {
    // Give the client 10 seconds to send the auth frame
    let timeout = tokio::time::Duration::from_secs(10);
    let result = tokio::time::timeout(timeout, receiver.next()).await;

    let frame = match result {
        Ok(Some(Ok(Message::Text(text)))) => text,
        _ => return None,
    };

    let msg: WsInbound = match serde_json::from_str(&frame) {
        Ok(m) => m,
        Err(_) => return None,
    };

    match msg {
        WsInbound::Auth { token } => validate_ws_token(&token, state).await,
        _ => None,
    }
}

async fn validate_ws_token(token: &str, state: &AppState) -> Option<Uuid> {
    crate::auth::validate_ws_token(token, &state.jwt_keys)
}

/// Push undelivered messages to a reconnecting client, then mark them delivered.
async fn deliver_offline_messages(user_id: &Uuid, state: &AppState, tx: &ConnTx) {
    let rows = sqlx::query(
        r#"
        SELECT m.id, m.conversation_id, m.sender_id, m.ciphertext, m.ek_public, m.opk_id, m.created_at
        FROM messages m
        JOIN dm_participants dp ON m.conversation_id = dp.conversation_id AND dp.user_id = $1
        WHERE m.delivered = FALSE
          AND m.sender_id != $1
          AND m.ciphertext IS NOT NULL
          AND m.deleted_at IS NULL
        ORDER BY m.created_at ASC
        LIMIT 100
        "#,
    )
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await;

    let Ok(rows) = rows else { return };
    if rows.is_empty() {
        return;
    }

    let mut delivered_ids: Vec<Uuid> = Vec::new();

    for row in &rows {
        let Ok(msg_id) = row.try_get::<Uuid, _>("id") else {
            continue;
        };
        let Ok(conv_id) = row.try_get::<Uuid, _>("conversation_id") else {
            continue;
        };
        let Ok(sender_id) = row.try_get::<Uuid, _>("sender_id") else {
            continue;
        };
        let Ok(cipher) = row.try_get::<Vec<u8>, _>("ciphertext") else {
            continue;
        };
        let ek: Option<Vec<u8>> = row.try_get("ek_public").ok().flatten();
        let opk_id: Option<i32> = row.try_get("opk_id").ok().flatten();

        let payload = serde_json::json!({
            "type": "dm",
            "id": msg_id,
            "conversation_id": conv_id,
            "sender_id": sender_id,
            "ciphertext": BASE64.encode(&cipher),
            "ephemeral_key": ek.as_ref().map(|k| BASE64.encode(k)),
            "opk_id": opk_id,
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

async fn handle_inbound(text: String, user_id: Uuid, state: &AppState, tx: &ConnTx) {
    // Max frame size: 64KB
    if text.len() > 65_536 {
        let _ = tx.send(WsOutbound::Error {
            code: 4003,
            message: "Frame too large".to_string(),
        });
        return;
    }

    let msg: WsInbound = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(_) => {
            let _ = tx.send(WsOutbound::Error {
                code: 4000,
                message: "Invalid message format".to_string(),
            });
            return;
        }
    };

    match msg {
        WsInbound::Ping => {
            let _ = tx.send(WsOutbound::Pong);
        }
        WsInbound::SendDm {
            conversation_id,
            ciphertext,
            ephemeral_key,
            opk_id,
            msg_num,
        } => {
            if !state.hub.check_msg_rate(&user_id) {
                let _ = tx.send(WsOutbound::Error {
                    code: 4029,
                    message: "Message rate limit exceeded".to_string(),
                });
                return;
            }
            handle_send_dm(
                conversation_id,
                ciphertext,
                ephemeral_key,
                opk_id,
                msg_num,
                user_id,
                state,
                tx,
            )
            .await;
        }
        WsInbound::TypingStart { channel_id } => {
            // Full implementation in S5
            tracing::debug!("User {user_id} typing in channel {channel_id}");
        }
        WsInbound::Read {
            message_id,
            channel_id,
        } => {
            // Full implementation in S5
            tracing::debug!("User {user_id} read message {message_id} in channel {channel_id}");
        }
        WsInbound::Reauth { token } => {
            if validate_ws_token(&token, state).await.is_none() {
                let _ = tx.send(WsOutbound::Error {
                    code: 4001,
                    message: "Invalid token".to_string(),
                });
            }
        }
        WsInbound::Auth { .. } => {
            // Already authenticated — ignore duplicate auth frames
        }
    }
}

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
    let cipher_bytes = match BASE64.decode(&ciphertext) {
        Ok(b) => b,
        Err(_) => {
            let _ = tx.send(WsOutbound::Error {
                code: 4005,
                message: "Invalid ciphertext encoding".into(),
            });
            return;
        }
    };
    let ek_bytes = ephemeral_key.as_ref().and_then(|k| BASE64.decode(k).ok());

    // Verify sender is a participant
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
        let _ = tx.send(WsOutbound::Error {
            code: 4006,
            message: "Not a participant in this conversation".into(),
        });
        return;
    }

    // Get recipient
    let recipient_row = sqlx::query(
        "SELECT user_id FROM dm_participants WHERE conversation_id = $1 AND user_id != $2 LIMIT 1",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .fetch_optional(state.db.pool())
    .await;

    let recipient_id: Uuid = match recipient_row {
        Ok(Some(row)) => match row.try_get("user_id") {
            Ok(id) => id,
            Err(_) => return,
        },
        _ => return,
    };

    let recipient_online = state.hub.is_online(&recipient_id);
    let msg_id = Uuid::new_v4();

    // Store ciphertext only — server NEVER holds plaintext for E2EE messages
    let store_result = sqlx::query(
        r#"
        INSERT INTO messages
            (id, conversation_id, sender_id, ciphertext, ek_public, opk_id, delivered)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(msg_id)
    .bind(conversation_id)
    .bind(sender_id)
    .bind(&cipher_bytes)
    .bind(ek_bytes.as_deref())
    .bind(opk_id)
    .bind(recipient_online) // mark delivered if recipient is online right now
    .execute(state.db.pool())
    .await;

    if store_result.is_err() {
        let _ = tx.send(WsOutbound::Error {
            code: 4007,
            message: "Failed to store message".into(),
        });
        return;
    }

    // Route to recipient if online
    let payload = serde_json::json!({
        "type": "dm",
        "id": msg_id,
        "conversation_id": conversation_id,
        "sender_id": sender_id,
        "ciphertext": ciphertext,
        "ephemeral_key": ephemeral_key,
        "opk_id": opk_id,
        "msg_num": msg_num,
    });
    state
        .hub
        .send_to_user(&recipient_id, WsOutbound::Message { payload });
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
        // Should not panic even if user is not connected
        hub.send_to_user(&user_id, WsOutbound::Pong);
    }
}
