use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;

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
}

impl Hub {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    pub fn register(&self, user_id: Uuid, conn_id: ConnectionId, tx: ConnTx) {
        self.connections
            .entry(user_id)
            .or_insert_with(DashMap::new)
            .insert(conn_id, tx);
    }

    pub fn unregister(&self, user_id: &Uuid, conn_id: &ConnectionId) {
        if let Some(user_conns) = self.connections.get(user_id) {
            user_conns.remove(conn_id);
            if user_conns.is_empty() {
                drop(user_conns);
                self.connections.remove(user_id);
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
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
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
                        // Axum handles Pong automatically, but we also send our own
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
        let _ = sqlx::query!(
            "UPDATE users SET last_seen_at = NOW() WHERE id = $1",
            uid
        )
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
        WsInbound::Auth { token } => {
            validate_ws_token(&token, state).await
        }
        _ => None,
    }
}

async fn validate_ws_token(token: &str, state: &AppState) -> Option<Uuid> {
    // Delegate to auth module's JWT validation
    crate::auth::validate_access_token(token, state.db.pool())
        .await
        .ok()
        .map(|claims| claims.sub)
}

async fn deliver_offline_messages(user_id: &Uuid, state: &AppState, tx: &ConnTx) {
    // Query undelivered messages and push them, then mark as delivered
    // Placeholder — full implementation in Phase 3 (messages module)
    let _ = (user_id, state, tx);
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
        WsInbound::TypingStart { channel_id } => {
            // Fan out to channel members — full implementation in Phase 6
            tracing::debug!("User {user_id} typing in channel {channel_id}");
        }
        WsInbound::Read { message_id, channel_id } => {
            // Upsert read receipt + fan out — full implementation in Phase 6
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
