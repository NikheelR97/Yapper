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
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    RateLimiter,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{num::NonZeroU32, sync::Arc};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::{constants, devices::DeviceTrustState, AppState};

/// Max frame size: 64KB (Rule 3).
const MAX_WS_FRAME_SIZE: usize = 64 * 1024;
/// Max server members to fan out a channel message to (Rule 2 / Rule 3).
const MAX_FANOUT_MEMBERS: i64 = 500;
/// Max concurrent WebSocket connections per user (prevents memory exhaustion).
const MAX_CONNECTIONS_PER_USER: usize = 5;
/// Max queued outbound messages per socket before the connection is dropped.
const MAX_OUTBOUND_QUEUE: usize = 256;

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
pub type ConnTx = mpsc::Sender<WsOutbound>;

#[derive(Clone)]
pub struct ConnectionHandle {
    tx: ConnTx,
    close_tx: watch::Sender<bool>,
}

#[derive(Clone)]
struct ConnectionMeta {
    user_id: Uuid,
    device_id: Option<Uuid>,
    route_user_level: bool,
    close_tx: watch::Sender<bool>,
}

/// The hub holds all active connections indexed by UserId -> ConnectionId -> Sender.
/// DashMap gives lock-free concurrent reads (critical for high fan-out).
pub struct Hub {
    /// user_id -> map of connection_id -> sender
    connections: DashMap<Uuid, DashMap<ConnectionId, ConnectionHandle>>,
    device_connections: DashMap<Uuid, DashMap<ConnectionId, ConnectionHandle>>,
    connection_meta: DashMap<ConnectionId, ConnectionMeta>,
    /// Per-user message rate limiters — cleaned up on full disconnect.
    msg_limiters: DashMap<Uuid, MsgRateLimiter>,
    /// Typing auto-stop timers keyed by (channel_id, user_id). Aborted on new TypingStart.
    pub typing_timers: DashMap<(Uuid, Uuid), tokio::task::JoinHandle<()>>,
    /// Away inactivity timers — fires after 5 min without a real user message.
    away_timers: DashMap<Uuid, tokio::task::JoinHandle<()>>,
    /// Set of users currently marked as "away" (connected but inactive).
    away_users: DashMap<Uuid, ()>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            device_connections: DashMap::new(),
            connection_meta: DashMap::new(),
            msg_limiters: DashMap::new(),
            typing_timers: DashMap::new(),
            away_timers: DashMap::new(),
            away_users: DashMap::new(),
        }
    }

    /// Returns true if the user is within their message rate limit (5/sec, burst 20).
    fn check_msg_rate(&self, user_id: &Uuid) -> bool {
        let limiter = self
            .msg_limiters
            .entry(*user_id)
            .or_insert_with(|| {
                // SAFETY: Literal 5 and 20 are non-zero; NonZeroU32::new cannot fail.
                let quota = governor::Quota::per_second(NonZeroU32::new(5).unwrap())
                    .allow_burst(NonZeroU32::new(20).unwrap());
                Arc::new(RateLimiter::direct(quota))
            })
            .clone();
        limiter.check().is_ok()
    }

    /// Register a new connection. Returns `false` if the user already has
    /// `MAX_CONNECTIONS_PER_USER` active connections (caller should close).
    pub fn register(
        &self,
        user_id: Uuid,
        device_id: Option<Uuid>,
        route_user_level: bool,
        conn_id: ConnectionId,
        handle: ConnectionHandle,
    ) -> bool {
        if route_user_level {
            let user_conns = self.connections.entry(user_id).or_default();
            if user_conns.len() >= MAX_CONNECTIONS_PER_USER {
                return false;
            }
            user_conns.insert(conn_id.clone(), handle.clone());
        }
        if let Some(device_id) = device_id {
            self.device_connections
                .entry(device_id)
                .or_default()
                .insert(conn_id.clone(), handle.clone());
        }
        self.connection_meta.insert(
            conn_id,
            ConnectionMeta {
                user_id,
                device_id,
                route_user_level,
                close_tx: handle.close_tx,
            },
        );
        true
    }

    pub fn unregister(&self, user_id: &Uuid, device_id: Option<&Uuid>, conn_id: &ConnectionId) {
        self.connection_meta.remove(conn_id);
        if let Some(user_conns) = self.connections.get(user_id) {
            user_conns.remove(conn_id);
            if user_conns.is_empty() {
                drop(user_conns);
                self.connections.remove(user_id);
                self.msg_limiters.remove(user_id);
                // Clean up away state — offline broadcast supersedes away
                if let Some((_, h)) = self.away_timers.remove(user_id) {
                    h.abort();
                }
                self.away_users.remove(user_id);
            }
        }
        if let Some(device_id) = device_id {
            if let Some(device_conns) = self.device_connections.get(device_id) {
                device_conns.remove(conn_id);
                if device_conns.is_empty() {
                    drop(device_conns);
                    self.device_connections.remove(device_id);
                }
            }
        }
    }

    fn disconnect_connection(&self, conn_id: &ConnectionId) {
        let Some((conn_id, meta)) = self.connection_meta.remove(conn_id) else {
            return;
        };

        let _ = meta.close_tx.send(true);

        if meta.route_user_level {
            if let Some(user_conns) = self.connections.get(&meta.user_id) {
                user_conns.remove(&conn_id);
                if user_conns.is_empty() {
                    drop(user_conns);
                    self.connections.remove(&meta.user_id);
                    self.msg_limiters.remove(&meta.user_id);
                    if let Some((_, h)) = self.away_timers.remove(&meta.user_id) {
                        h.abort();
                    }
                    self.away_users.remove(&meta.user_id);
                }
            }
        }

        if let Some(device_id) = meta.device_id {
            if let Some(device_conns) = self.device_connections.get(&device_id) {
                device_conns.remove(&conn_id);
                if device_conns.is_empty() {
                    drop(device_conns);
                    self.device_connections.remove(&device_id);
                }
            }
        }
    }

    /// Reset (or start) the 5-min away inactivity timer for a user.
    /// If the user was previously marked away, immediately broadcasts them as active again.
    pub fn reset_away_timer(&self, user_id: Uuid, state: AppState) {
        // Cancel existing timer
        if let Some((_, h)) = self.away_timers.remove(&user_id) {
            h.abort();
        }

        // If the user was away, clear the flag and broadcast "back"
        if self.away_users.remove(&user_id).is_some() {
            let state_back = state.clone();
            tokio::spawn(async move {
                broadcast_presence(user_id, true, None, false, &state_back).await;
            });
        }

        // Spawn new inactivity timer — fires after 5 min of silence
        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;
            // Mark as away in the shared hub (Hub is behind Arc<Hub> in AppState)
            state.hub.away_users.insert(user_id, ());
            broadcast_presence(user_id, true, None, true, &state).await;
        });
        self.away_timers.insert(user_id, handle);
    }

    pub fn is_online(&self, user_id: &Uuid) -> bool {
        self.connections
            .get(user_id)
            .map(|m| !m.is_empty())
            .unwrap_or(false)
    }

    pub fn is_device_online(&self, device_id: &Uuid) -> bool {
        self.device_connections
            .get(device_id)
            .map(|m| !m.is_empty())
            .unwrap_or(false)
    }

    pub fn is_away(&self, user_id: &Uuid) -> bool {
        self.away_users.contains_key(user_id)
    }

    /// Send a message to all connections of a specific user.
    pub fn send_to_user(&self, user_id: &Uuid, msg: WsOutbound) {
        if let Some(user_conns) = self.connections.get(user_id) {
            let mut stale = Vec::new();
            for entry in user_conns.iter() {
                if entry.value().tx.try_send(msg.clone()).is_err() {
                    stale.push(entry.key().clone());
                }
            }
            drop(user_conns);
            for conn_id in stale {
                self.disconnect_connection(&conn_id);
            }
        }
    }

    pub fn send_to_device(&self, device_id: &Uuid, msg: WsOutbound) {
        if let Some(device_conns) = self.device_connections.get(device_id) {
            let mut stale = Vec::new();
            for entry in device_conns.iter() {
                if entry.value().tx.try_send(msg.clone()).is_err() {
                    stale.push(entry.key().clone());
                }
            }
            drop(device_conns);
            for conn_id in stale {
                self.disconnect_connection(&conn_id);
            }
        }
    }

    /// Fan out a message to multiple users (e.g. all members of a channel).
    pub fn broadcast(&self, user_ids: &[Uuid], msg: WsOutbound) {
        for user_id in user_ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
            self.send_to_user(user_id, msg.clone());
        }
    }

    /// Count how many of the given user_ids have at least one active WS connection.
    /// Used by music skip-vote threshold calculation.
    pub fn count_online(&self, user_ids: &[Uuid]) -> usize {
        user_ids
            .iter()
            .take(MAX_FANOUT_MEMBERS as usize)
            .filter(|uid| self.connections.contains_key(uid))
            .count()
    }
}

/// Messages sent FROM client TO server over WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsInbound {
    Auth {
        token: String,
    },
    Reauth {
        token: String,
    },
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
    TypingStart {
        channel_id: Uuid,
    },
    Read {
        message_id: Uuid,
        channel_id: Uuid,
    },
    Ping,
}

/// Messages sent FROM server TO client over WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsOutbound {
    Ready {
        user_id: Uuid,
    },
    Message {
        payload: serde_json::Value,
    },
    Typing {
        channel_id: Uuid,
        user_id: Uuid,
    },
    TypingStop {
        channel_id: Uuid,
        user_id: Uuid,
    },
    ReadReceipt {
        message_id: Uuid,
        user_id: Uuid,
    },
    /// Real-time presence notification. `online=false` means disconnected;
    /// `online=true, away=true` means connected but inactive for 5+ min.
    Presence {
        user_id: Uuid,
        online: bool,
        away: bool,
        last_seen_at: Option<String>,
    },
    ReAuthRequired,
    CanvasUpdate {
        payload: serde_json::Value,
    },
    ParentNotification {
        payload: serde_json::Value,
    },
    Error {
        code: u16,
        message: String,
    },
    Pong,
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn send_ws_error(tx: &ConnTx, code: u16, message: &str) {
    // SAFETY: If the channel is full or closed the client is already disconnecting;
    // the error frame is best-effort and dropping it is acceptable.
    let _ = tx.try_send(WsOutbound::Error {
        code,
        message: message.to_string(),
    });
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
    let (sender, mut receiver) = socket.split();
    let conn_id = ConnectionId::new();
    let (tx, rx) = mpsc::channel::<WsOutbound>(MAX_OUTBOUND_QUEUE);
    let (close_tx, close_rx) = watch::channel(false);

    let mut send_task = spawn_ws_send_task(sender, rx, close_rx);

    let auth = match wait_for_auth(&mut receiver, &state).await {
        Some(auth) => auth,
        None => { send_task.abort(); return; }
    };

    let handle = ConnectionHandle { tx: tx.clone(), close_tx: close_tx.clone() };
    let is_trusted = auth.trust_state != Some(DeviceTrustState::PendingTrust);
    if !state.hub.register(auth.user_id, auth.device_id, is_trusted, conn_id.clone(), handle) {
        send_ws_error(&tx, 4008, "Too many connections");
        send_task.abort();
        return;
    }

    let _ = tx.try_send(WsOutbound::Ready { user_id: auth.user_id });
    deliver_on_connect(&auth, is_trusted, &state, &tx).await;

    run_receive_loop(&mut receiver, &mut send_task, auth.user_id, auth.device_id, &state, &tx)
        .await;

    state.hub.unregister(&auth.user_id, auth.device_id.as_ref(), &conn_id);
    update_last_seen(auth.user_id, &state);

    if is_trusted && !state.hub.is_online(&auth.user_id) {
        let last_seen = chrono::Utc::now().to_rfc3339();
        broadcast_presence(auth.user_id, false, Some(last_seen), false, &state).await;
    }
}

fn spawn_ws_send_task(
    mut sender: futures::stream::SplitSink<WebSocket, Message>,
    mut rx: mpsc::Receiver<WsOutbound>,
    mut close_rx: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = close_rx.changed() => {
                    if changed.is_err() || *close_rx.borrow() { break; }
                }
                msg = rx.recv() => {
                    let Some(msg) = msg else { break; };
                    let Ok(text) = serde_json::to_string(&msg) else {
                        tracing::error!("WS serialize error");
                        continue;
                    };
                    if sender.send(Message::Text(text)).await.is_err() { break; }
                }
            }
        }
    })
}

async fn deliver_on_connect(auth: &WsAuth, is_trusted: bool, state: &AppState, tx: &ConnTx) {
    if is_trusted {
        deliver_offline_messages(&auth.user_id, auth.device_id.as_ref(), state, tx).await;
        deliver_pending_key_dists(&auth.user_id, auth.device_id.as_ref(), state, tx).await;
        broadcast_presence(auth.user_id, true, None, false, state).await;
        state.hub.reset_away_timer(auth.user_id, state.clone());
    }
    deliver_pending_sync_events(auth.device_id.as_ref(), state, tx).await;
}

async fn run_receive_loop(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    send_task: &mut tokio::task::JoinHandle<()>,
    user_id: Uuid,
    device_id: Option<Uuid>,
    state: &AppState,
    tx: &ConnTx,
) {
    let mut processed = 0usize;
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_inbound(text, user_id, device_id, state, tx).await;
                        processed += 1;
                        if processed >= constants::MAX_MESSAGES_PER_TICK {
                            processed = 0;
                            tokio::task::yield_now().await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(_))) => { let _ = tx.try_send(WsOutbound::Pong); }
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
        if let Err(e) = sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
        {
            tracing::warn!(user_id = %user_id, "Failed to update last_seen_at: {e}");
        }
    });
}

// ─── Presence broadcast ──────────────────────────────────────────────────────

/// Fan out a Presence event to all users who share a DM or server with `user_id`.
/// This covers everyone who might display an avatar with a presence dot.
async fn broadcast_presence(
    user_id: Uuid,
    online: bool,
    last_seen_at: Option<String>,
    away: bool,
    state: &AppState,
) {
    debug_assert!(user_id != Uuid::nil());

    // Collect unique peer IDs from:
    //  1. DM conversations
    //  2. Server memberships (all co-members)
    let peer_ids: Vec<Uuid> = {
        let dm_rows = sqlx::query(
            "SELECT user_id FROM dm_participants \
             WHERE conversation_id IN (\
               SELECT conversation_id FROM dm_participants WHERE user_id = $1\
             ) AND user_id != $1 LIMIT 500",
        )
        .bind(user_id)
        .fetch_all(state.db.pool())
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(user_id = %user_id, "Failed to fetch DM peers for presence fanout: {e}");
            vec![]
        });

        let server_rows = sqlx::query(
            "SELECT user_id FROM server_memberships \
             WHERE server_id IN (\
               SELECT server_id FROM server_memberships WHERE user_id = $1\
             ) AND user_id != $1 \
             LIMIT $2",
        )
        .bind(user_id)
        .bind(MAX_FANOUT_MEMBERS)
        .fetch_all(state.db.pool())
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(user_id = %user_id, "Failed to fetch server peers for presence fanout: {e}");
            vec![]
        });

        let mut ids: Vec<Uuid> = dm_rows
            .iter()
            .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
            .chain(
                server_rows
                    .iter()
                    .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok()),
            )
            .collect();
        ids.sort();
        ids.dedup();
        ids
    };

    if peer_ids.is_empty() {
        return;
    }

    let msg = WsOutbound::Presence {
        user_id,
        online,
        away,
        last_seen_at: last_seen_at.clone(),
    };

    state.hub.broadcast(&peer_ids, msg);
}

// ─── Auth ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct WsAuth {
    user_id: Uuid,
    device_id: Option<Uuid>,
    trust_state: Option<DeviceTrustState>,
}

async fn wait_for_auth(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    state: &AppState,
) -> Option<WsAuth> {
    let timeout = tokio::time::Duration::from_secs(5);
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

async fn validate_ws_token(token: &str, state: &AppState) -> Option<WsAuth> {
    let claims = crate::auth::service::validate_access_token(token, &state.jwt_keys)
        .ok()?
        .claims;

    let mut trust_state = None;
    if let Some(device_id) = claims.device_id {
        let device = crate::devices::get_device_for_user(claims.sub, device_id, state)
            .await
            .ok()?;
        if device.revoked_at.is_some() || device.trust_state == DeviceTrustState::Revoked {
            return None;
        }
        trust_state = Some(device.trust_state);
    }

    Some(WsAuth {
        user_id: claims.sub,
        device_id: claims.device_id,
        trust_state,
    })
}

async fn live_device_trust_state(
    user_id: Uuid,
    device_id: Uuid,
    state: &AppState,
) -> Option<DeviceTrustState> {
    let device = crate::devices::get_device_for_user(user_id, device_id, state)
        .await
        .ok()?;
    if device.revoked_at.is_some() || device.trust_state == DeviceTrustState::Revoked {
        return None;
    }
    Some(device.trust_state)
}

// ─── Offline delivery ────────────────────────────────────────────────────────

async fn deliver_offline_messages(
    user_id: &Uuid,
    device_id: Option<&Uuid>,
    state: &AppState,
    tx: &ConnTx,
) {
    debug_assert!(*user_id != Uuid::nil());

    if let Some(device_id) = device_id {
        deliver_offline_envelopes(device_id, state, tx).await;
    } else {
        deliver_offline_legacy(user_id, state, tx).await;
    }
}

/// Deliver per-device DM envelopes (v2 multi-device path).
async fn deliver_offline_envelopes(
    device_id: &Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
    let rows = sqlx::query(
        r#"
        SELECT e.id AS envelope_id,
               m.id,
               m.conversation_id,
               m.sender_id,
               m.sender_device_id,
               sd.signal_device_id AS sender_signal_device_id,
               e.recipient_device_id,
               e.ciphertext,
               e.ek_public,
               e.opk_id,
               e.msg_num
        FROM dm_message_envelopes e
        JOIN messages m ON m.id = e.message_id
        JOIN devices sd ON sd.id = m.sender_device_id
        WHERE e.recipient_device_id = $1
          AND e.delivered_at IS NULL
          AND m.deleted_at IS NULL
        ORDER BY m.created_at ASC
        LIMIT 100
        "#,
    )
    .bind(device_id)
    .fetch_all(state.db.pool())
    .await;

    let Ok(rows) = rows else { return };
    if rows.is_empty() {
        return;
    }

    let mut delivered_ids: Vec<Uuid> = Vec::with_capacity(rows.len());
    for row in &rows {
        let Ok(envelope_id) = row.try_get::<Uuid, _>("envelope_id") else {
            continue;
        };
        let Ok(msg_id) = row.try_get::<Uuid, _>("id") else {
            continue;
        };
        let Ok(conv_id) = row.try_get::<Uuid, _>("conversation_id") else {
            continue;
        };
        let Ok(sender_id) = row.try_get::<Uuid, _>("sender_id") else {
            continue;
        };
        let Ok(sender_device_id) = row.try_get::<Uuid, _>("sender_device_id") else {
            continue;
        };
        let Ok(sender_signal_device_id) = row.try_get::<i32, _>("sender_signal_device_id")
        else {
            continue;
        };
        let Ok(recipient_device_id) = row.try_get::<Uuid, _>("recipient_device_id") else {
            continue;
        };
        let Ok(cipher) = row.try_get::<Vec<u8>, _>("ciphertext") else {
            continue;
        };
        let ek: Option<Vec<u8>> = row.try_get("ek_public").ok().flatten();
        let opk_id: Option<i32> = row.try_get("opk_id").ok().flatten();
        let msg_num: i32 = row.try_get("msg_num").unwrap_or(0);

        let payload = serde_json::json!({
            "type": "dm_v2",
            "id": msg_id,
            "conversation_id": conv_id,
            "sender_id": sender_id,
            "sender_device_id": sender_device_id,
            "sender_signal_device_id": sender_signal_device_id,
            "recipient_device_id": recipient_device_id,
            "ciphertext": BASE64.encode(&cipher),
            "ephemeral_key": ek.as_ref().map(|k| BASE64.encode(k)),
            "opk_id": opk_id,
            "msg_num": msg_num,
        });

        if tx.try_send(WsOutbound::Message { payload }).is_ok() {
            delivered_ids.push(envelope_id);
        }
    }

    if !delivered_ids.is_empty() {
        if let Err(e) = sqlx::query(
            "UPDATE dm_message_envelopes SET delivered_at = NOW() WHERE id = ANY($1)",
        )
        .bind(&delivered_ids)
        .execute(state.db.pool())
        .await
        {
            tracing::warn!("Failed to mark DM envelopes delivered: {e}");
        }
    }
}

/// Deliver legacy DM messages (pre-multi-device path, no device_id).
async fn deliver_offline_legacy(
    user_id: &Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
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
    if rows.is_empty() {
        return;
    }

    let mut delivered_ids: Vec<Uuid> = Vec::with_capacity(rows.len());

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
            "type": "dm", "id": msg_id, "conversation_id": conv_id,
            "sender_id": sender_id, "ciphertext": BASE64.encode(&cipher),
            "ephemeral_key": ek.as_ref().map(|k| BASE64.encode(k)), "opk_id": opk_id,
        });

        if tx.try_send(WsOutbound::Message { payload }).is_ok() {
            delivered_ids.push(msg_id);
        }
    }

    if !delivered_ids.is_empty() {
        if let Err(e) = sqlx::query("UPDATE messages SET delivered = TRUE WHERE id = ANY($1)")
            .bind(&delivered_ids)
            .execute(state.db.pool())
            .await
        {
            tracing::warn!("Failed to mark messages delivered: {e}");
        }
    }
}

// ─── Offline sender key distributions ────────────────────────────────────────

async fn deliver_pending_key_dists(
    user_id: &Uuid,
    device_id: Option<&Uuid>,
    state: &AppState,
    tx: &ConnTx,
) {
    debug_assert!(*user_id != Uuid::nil());

    let rows = if let Some(device_id) = device_id {
        sqlx::query(
            "UPDATE sender_key_distributions \
             SET delivered = TRUE \
             WHERE delivered = FALSE \
               AND (to_device_id = $1 OR (to_device_id IS NULL AND to_user = $2)) \
             RETURNING channel_id, from_user, from_device_id, ciphertext, ek_public",
        )
        .bind(device_id)
        .bind(user_id)
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "UPDATE sender_key_distributions \
             SET delivered = TRUE \
             WHERE to_user = $1 AND delivered = FALSE \
             RETURNING channel_id, from_user, from_device_id, ciphertext, ek_public",
        )
        .bind(user_id)
        .fetch_all(state.db.pool())
        .await
    };

    let Ok(rows) = rows else { return };

    for row in &rows {
        let Ok(channel_id) = row.try_get::<uuid::Uuid, _>("channel_id") else {
            continue;
        };
        let Ok(from_user) = row.try_get::<uuid::Uuid, _>("from_user") else {
            continue;
        };
        let from_device_id: Option<uuid::Uuid> = row.try_get("from_device_id").ok().flatten();
        let Ok(ct) = row.try_get::<Vec<u8>, _>("ciphertext") else {
            continue;
        };
        let Ok(ek) = row.try_get::<Vec<u8>, _>("ek_public") else {
            continue;
        };

        let payload = serde_json::json!({
            "type": "key_dist_v2",
            "channel_id": channel_id,
            "from_user": from_user,
            "from_device_id": from_device_id,
            "ciphertext": BASE64.encode(&ct),
            "ek_public":  BASE64.encode(&ek),
        });
        if tx.try_send(WsOutbound::Message { payload }).is_err() {
            tracing::debug!("Recipient disconnected during key dist delivery");
        }
    }
}

async fn deliver_pending_sync_events(device_id: Option<&Uuid>, state: &AppState, tx: &ConnTx) {
    let Some(device_id) = device_id else {
        return;
    };

    let Ok(events) = crate::devices::take_sync_events(*device_id, state).await else {
        return;
    };

    for event in &events {
        let payload = crate::devices::sync_event_payload(event);
        if tx.try_send(WsOutbound::Message { payload }).is_err() {
            tracing::debug!("Device disconnected during sync event delivery");
            break;
        }
    }
}

// ─── Inbound dispatch ────────────────────────────────────────────────────────

async fn handle_inbound(
    text: String,
    user_id: Uuid,
    device_id: Option<Uuid>,
    state: &AppState,
    tx: &ConnTx,
) {
    if text.len() > MAX_WS_FRAME_SIZE {
        send_ws_error(tx, 4003, "Frame too large");
        return;
    }

    let msg: WsInbound = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(_) => {
            send_ws_error(tx, 4000, "Invalid message format");
            return;
        }
    };

    let is_control_message = matches!(
        &msg,
        WsInbound::Ping | WsInbound::Auth { .. } | WsInbound::Reauth { .. }
    );

    let device_is_trusted = match device_id {
        Some(device_id) => match live_device_trust_state(user_id, device_id, state).await {
            Some(DeviceTrustState::Trusted) => true,
            Some(DeviceTrustState::PendingTrust) => false,
            Some(DeviceTrustState::Revoked) | None => {
                send_ws_error(tx, 4001, "Device revoked");
                return;
            }
        },
        None => true,
    };

    if !device_is_trusted && !is_control_message {
        send_ws_error(tx, 4006, "Device approval required");
        return;
    }

    // Reset the away timer on any real user activity. Pings and auth frames are
    // automatic/background — only deliberate actions count as "presence".
    if device_is_trusted && !is_control_message {
        state.hub.reset_away_timer(user_id, state.clone());
    }

    match msg {
        WsInbound::Ping => {
            let _ = tx.try_send(WsOutbound::Pong);
        }
        WsInbound::SendDm {
            conversation_id,
            ciphertext,
            ephemeral_key,
            opk_id,
            msg_num,
        } => {
            if check_rate_limit(state, &user_id, tx) {
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
        }
        WsInbound::SendChannel {
            channel_id,
            ciphertext,
            message_type,
            msg_num,
        } => {
            if check_rate_limit(state, &user_id, tx) {
                handle_send_channel(
                    channel_id,
                    ciphertext,
                    message_type,
                    msg_num,
                    user_id,
                    device_id,
                    state,
                    tx,
                )
                .await;
            }
        }
        WsInbound::TypingStart { channel_id } => {
            handle_typing_start(channel_id, user_id, state).await;
        }
        WsInbound::Read {
            message_id,
            channel_id,
        } => {
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

    if ciphertext.len() > constants::MAX_MESSAGE_LENGTH {
        send_ws_error(tx, 4010, "Message too large");
        return;
    }

    let cipher_bytes = match BASE64.decode(&ciphertext) {
        Ok(b) => b,
        Err(_) => {
            send_ws_error(tx, 4005, "Invalid ciphertext encoding");
            return;
        }
    };
    let ek_bytes = ephemeral_key.as_ref().and_then(|k| BASE64.decode(k).ok());

    let recipient_id = match resolve_dm_recipient(conversation_id, sender_id, state, tx).await {
        Some(id) => id,
        None => return,
    };

    store_and_route_dm(
        DmContext {
            conversation_id,
            cipher_bytes: &cipher_bytes,
            ek_bytes: ek_bytes.as_deref(),
            opk_id,
            msg_num,
            ciphertext_b64: &ciphertext,
            ephemeral_key: &ephemeral_key,
            sender_id,
            recipient_id,
        },
        state,
        tx,
    )
    .await;
}

async fn resolve_dm_recipient(
    conversation_id: Uuid,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) -> Option<Uuid> {
    let is_participant =
        sqlx::query("SELECT 1 FROM dm_participants WHERE conversation_id = $1 AND user_id = $2")
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

    let recipient_id: Uuid = match recipient_row {
        Ok(Some(row)) => match row.try_get("user_id").ok() {
            Some(id) => id,
            None => return None,
        },
        _ => return None,
    };

    // Prevent DMs to bot accounts
    let is_bot = sqlx::query("SELECT account_type FROM users WHERE id = $1")
        .bind(recipient_id)
        .fetch_optional(state.db.pool())
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<String, _>("account_type").ok())
        .map(|t| t == "bot")
        .unwrap_or(false);

    if is_bot {
        send_ws_error(tx, 4011, "Cannot send DMs to bot accounts");
        return None;
    }

    Some(recipient_id)
}

struct DmContext<'a> {
    conversation_id: Uuid,
    cipher_bytes: &'a [u8],
    ek_bytes: Option<&'a [u8]>,
    opk_id: Option<i32>,
    msg_num: u32,
    ciphertext_b64: &'a str,
    ephemeral_key: &'a Option<String>,
    sender_id: Uuid,
    recipient_id: Uuid,
}

async fn store_and_route_dm(dm: DmContext<'_>, state: &AppState, tx: &ConnTx) {
    let DmContext {
        conversation_id,
        cipher_bytes,
        ek_bytes,
        opk_id,
        msg_num,
        ciphertext_b64,
        ephemeral_key,
        sender_id,
        recipient_id,
    } = dm;
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
    state
        .hub
        .send_to_user(&recipient_id, WsOutbound::Message { payload });

    // Push notification to offline devices (best-effort, fire-and-forget)
    if !recipient_online {
        let state = state.clone();
        let recipient_id = recipient_id;
        let conversation_id = conversation_id;
        let sender_id = sender_id;
        tokio::spawn(async move {
            let mut meta = std::collections::HashMap::new();
            meta.insert("conversation_id".into(), conversation_id.to_string());
            meta.insert("sender_id".into(), sender_id.to_string());
            crate::notifications::notify_user_offline_devices(
                recipient_id, "dm", &meta, &state,
            )
            .await;
        });
    }
}

// ─── Send Channel ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_send_channel(
    channel_id: Uuid,
    ciphertext: String,
    message_type: Option<String>,
    msg_num: Option<i32>,
    sender_id: Uuid,
    sender_device_id: Option<Uuid>,
    state: &AppState,
    tx: &ConnTx,
) {
    debug_assert!(sender_id != Uuid::nil());

    if ciphertext.len() > constants::MAX_MESSAGE_LENGTH {
        send_ws_error(tx, 4010, "Message too large");
        return;
    }

    let server_id = match resolve_channel_membership(channel_id, sender_id, state, tx).await {
        Some(id) => id,
        None => return,
    };

    // Bot messages are stored as plaintext (bots don't participate in E2EE)
    let is_bot = is_bot_user(sender_id, state).await;

    if is_bot {
        let msg_type = message_type.as_deref().unwrap_or("text");
        store_and_fanout_bot_channel(
            channel_id, server_id, &ciphertext, msg_type, sender_id, state, tx,
        )
        .await;
    } else {
        let cipher_bytes = match BASE64.decode(&ciphertext) {
            Ok(b) => b,
            Err(_) => {
                send_ws_error(tx, 4005, "Invalid ciphertext encoding");
                return;
            }
        };
        let msg_type = message_type.as_deref().unwrap_or("text");
        store_and_fanout_channel(
            channel_id, server_id, &cipher_bytes, msg_type, msg_num, &ciphertext,
            sender_id, sender_device_id, state, tx,
        )
        .await;
    }
}

async fn is_bot_user(user_id: Uuid, state: &AppState) -> bool {
    sqlx::query("SELECT account_type FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(state.db.pool())
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<String, _>("account_type").ok())
        .map(|t| t == "bot")
        .unwrap_or(false)
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
        None => {
            send_ws_error(tx, 4006, "Channel not found");
            return None;
        }
    };

    let is_member =
        sqlx::query("SELECT 1 FROM server_memberships WHERE user_id = $1 AND server_id = $2")
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

async fn store_and_fanout_bot_channel(
    channel_id: Uuid,
    server_id: Uuid,
    plaintext: &str,
    msg_type: &str,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) {
    let msg_id = Uuid::new_v4();

    let store_result = sqlx::query(
        "INSERT INTO messages (id, channel_id, sender_id, plaintext, message_type, delivered) \
         VALUES ($1, $2, $3, $4, $5, TRUE)",
    )
    .bind(msg_id)
    .bind(channel_id)
    .bind(sender_id)
    .bind(plaintext)
    .bind(msg_type)
    .execute(state.db.pool())
    .await;

    if let Err(e) = store_result {
        tracing::error!("Failed to store bot channel message: {e}");
        send_ws_error(tx, 4007, "Failed to store message");
        return;
    }

    fanout_to_channel_members(
        channel_id,
        server_id,
        serde_json::json!({
            "type": "channel", "id": msg_id, "channel_id": channel_id,
            "server_id": server_id, "sender_id": sender_id,
            "plaintext": plaintext, "message_type": msg_type, "is_bot": true,
        }),
        state,
    )
    .await;
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
    sender_device_id: Option<Uuid>,
    state: &AppState,
    tx: &ConnTx,
) {
    let msg_id = Uuid::new_v4();

    let store_result = sqlx::query(
        "INSERT INTO messages (id, channel_id, sender_id, sender_device_id, ciphertext, message_type, msg_num, delivered) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE)",
    )
    .bind(msg_id)
    .bind(channel_id)
    .bind(sender_id)
    .bind(sender_device_id)
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

    fanout_to_channel_members(
        channel_id,
        server_id,
        serde_json::json!({
            "type": "channel", "id": msg_id, "channel_id": channel_id,
            "server_id": server_id, "sender_id": sender_id,
            "sender_device_id": sender_device_id,
            "ciphertext": ciphertext_b64, "message_type": msg_type, "msg_num": msg_num,
        }),
        state,
    )
    .await;
}

async fn fanout_to_channel_members(
    channel_id: Uuid,
    server_id: Uuid,
    payload: serde_json::Value,
    state: &AppState,
) {
    debug_assert!(channel_id != Uuid::nil());
    let member_rows =
        match sqlx::query("SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2")
            .bind(server_id)
            .bind(MAX_FANOUT_MEMBERS)
            .fetch_all(state.db.pool())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("Failed to fetch members for fanout: {e}");
                return;
            }
        };

    for m in member_rows.iter().take(MAX_FANOUT_MEMBERS as usize) {
        if let Ok(uid) = m.try_get::<Uuid, _>("user_id") {
            state.hub.send_to_user(
                &uid,
                WsOutbound::Message {
                    payload: payload.clone(),
                },
            );
        }
    }
}

// ─── Typing indicators ────────────────────────────────────────────────────────

/// Resolves member UUIDs for a channel (bounded to MAX_FANOUT_MEMBERS).
async fn fetch_channel_member_ids(channel_id: Uuid, state: &AppState) -> Option<Vec<Uuid>> {
    debug_assert!(channel_id != Uuid::nil());

    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(state.db.pool())
        .await
        .ok()
        .flatten()?;
    let server_id: Uuid = server_row.try_get("server_id").ok()?;

    let rows = sqlx::query("SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2")
        .bind(server_id)
        .bind(MAX_FANOUT_MEMBERS)
        .fetch_all(state.db.pool())
        .await
        .ok()?;

    Some(
        rows.iter()
            .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
            .collect(),
    )
}

/// Fan out a TypingStart event and schedule an auto-stop after 5 seconds of silence.
async fn handle_typing_start(channel_id: Uuid, user_id: Uuid, state: &AppState) {
    debug_assert!(user_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());

    let Some(member_ids) = fetch_channel_member_ids(channel_id, state).await else {
        return;
    };

    // Only members may broadcast typing indicators (S-002 audit fix).
    if !member_ids.contains(&user_id) {
        return;
    }

    let key = (channel_id, user_id);

    // Abort any existing timer to reset the 5-second window
    if let Some((_, old)) = state.hub.typing_timers.remove(&key) {
        old.abort();
    }

    for uid in member_ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
        if *uid != user_id {
            state.hub.send_to_user(
                uid,
                WsOutbound::Typing {
                    channel_id,
                    user_id,
                },
            );
        }
    }

    let hub = Arc::clone(&state.hub);
    let ids = member_ids;
    let handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        for uid in ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
            if *uid != user_id {
                hub.send_to_user(
                    uid,
                    WsOutbound::TypingStop {
                        channel_id,
                        user_id,
                    },
                );
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

    // Verify channel membership before processing read receipt
    let Some(member_ids) = fetch_channel_member_ids(channel_id, state).await else {
        return;
    };
    if !member_ids.contains(&user_id) {
        return; // Silently drop — user is not a channel member
    }

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

    for uid in member_ids.iter().take(MAX_FANOUT_MEMBERS as usize) {
        state.hub.send_to_user(
            uid,
            WsOutbound::ReadReceipt {
                message_id,
                user_id,
            },
        );
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
        let (tx, _rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx, _close_rx) = watch::channel(false);

        assert!(!hub.is_online(&user_id));
        assert!(hub.register(
            user_id,
            None,
            true,
            conn_id.clone(),
            ConnectionHandle { tx, close_tx },
        ));
        assert!(hub.is_online(&user_id));
        hub.unregister(&user_id, None, &conn_id);
        assert!(!hub.is_online(&user_id));
    }

    #[test]
    fn test_hub_max_connections_per_user() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        for _ in 0..MAX_CONNECTIONS_PER_USER {
            let conn_id = ConnectionId::new();
            let (tx, _rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
            let (close_tx, _close_rx) = watch::channel(false);
            assert!(hub.register(
                user_id,
                None,
                true,
                conn_id,
                ConnectionHandle { tx, close_tx },
            ));
        }
        // One more should be rejected
        let conn_id = ConnectionId::new();
        let (tx, _rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx, _close_rx) = watch::channel(false);
        assert!(!hub.register(
            user_id,
            None,
            true,
            conn_id,
            ConnectionHandle { tx, close_tx },
        ));
    }

    #[test]
    fn test_hub_send_to_offline_user_is_noop() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        hub.send_to_user(&user_id, WsOutbound::Pong);
    }

    // ─── Rate limiter tests ──────────────────────────────────────────────────

    #[test]
    fn test_check_msg_rate_allows_burst() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        // Burst of 20 should all pass
        for i in 0..20 {
            assert!(
                hub.check_msg_rate(&user_id),
                "message {i} within burst should be allowed"
            );
        }
    }

    #[test]
    fn test_check_msg_rate_rejects_after_burst() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        // Exhaust the burst of 20
        for _ in 0..20 {
            hub.check_msg_rate(&user_id);
        }
        // The 21st message should be rate-limited
        assert!(
            !hub.check_msg_rate(&user_id),
            "message after burst exhausted should be rejected"
        );
    }

    #[test]
    fn test_check_msg_rate_independent_per_user() {
        let hub = Hub::new();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        // Exhaust user_a's burst
        for _ in 0..20 {
            hub.check_msg_rate(&user_a);
        }
        assert!(
            !hub.check_msg_rate(&user_a),
            "user_a should be rate-limited"
        );
        // user_b should still be fine
        assert!(
            hub.check_msg_rate(&user_b),
            "user_b should not be affected by user_a's rate limit"
        );
    }

    #[test]
    fn test_rate_limiter_cleaned_up_on_unregister() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        let conn_id = ConnectionId::new();
        let (tx, _rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx, _close_rx) = watch::channel(false);
        hub.register(
            user_id,
            None,
            true,
            conn_id.clone(),
            ConnectionHandle { tx, close_tx },
        );

        // Trigger rate limiter creation
        hub.check_msg_rate(&user_id);
        assert!(hub.msg_limiters.contains_key(&user_id));

        // Unregister (last connection) should clean up
        hub.unregister(&user_id, None, &conn_id);
        assert!(
            !hub.msg_limiters.contains_key(&user_id),
            "rate limiter should be removed when user fully disconnects"
        );
    }

    // ─── WsInbound deserialization tests ─────────────────────────────────────

    #[test]
    fn test_ws_inbound_ping() {
        let json = r#"{"type":"ping"}"#;
        let msg: WsInbound = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WsInbound::Ping));
    }

    #[test]
    fn test_ws_inbound_auth() {
        let json = r#"{"type":"auth","token":"my.jwt.token"}"#;
        let msg: WsInbound = serde_json::from_str(json).unwrap();
        match msg {
            WsInbound::Auth { token } => assert_eq!(token, "my.jwt.token"),
            other => panic!("Expected Auth, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_reauth() {
        let json = r#"{"type":"reauth","token":"refreshed.jwt.token"}"#;
        let msg: WsInbound = serde_json::from_str(json).unwrap();
        match msg {
            WsInbound::Reauth { token } => assert_eq!(token, "refreshed.jwt.token"),
            other => panic!("Expected Reauth, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_send_dm_full() {
        let conv_id = Uuid::new_v4();
        let json = serde_json::json!({
            "type": "send_dm",
            "conversation_id": conv_id,
            "ciphertext": "AQID",
            "ephemeral_key": "BAUG",
            "opk_id": 42,
            "msg_num": 7
        });
        let msg: WsInbound = serde_json::from_value(json).unwrap();
        match msg {
            WsInbound::SendDm {
                conversation_id,
                ciphertext,
                ephemeral_key,
                opk_id,
                msg_num,
            } => {
                assert_eq!(conversation_id, conv_id);
                assert_eq!(ciphertext, "AQID");
                assert_eq!(ephemeral_key, Some("BAUG".to_string()));
                assert_eq!(opk_id, Some(42));
                assert_eq!(msg_num, 7);
            }
            other => panic!("Expected SendDm, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_send_dm_optional_fields_null() {
        let conv_id = Uuid::new_v4();
        let json = serde_json::json!({
            "type": "send_dm",
            "conversation_id": conv_id,
            "ciphertext": "AQID",
            "ephemeral_key": null,
            "opk_id": null,
            "msg_num": 0
        });
        let msg: WsInbound = serde_json::from_value(json).unwrap();
        match msg {
            WsInbound::SendDm {
                ephemeral_key,
                opk_id,
                msg_num,
                ..
            } => {
                assert_eq!(ephemeral_key, None);
                assert_eq!(opk_id, None);
                assert_eq!(msg_num, 0);
            }
            other => panic!("Expected SendDm, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_send_channel() {
        let ch_id = Uuid::new_v4();
        let json = serde_json::json!({
            "type": "send_channel",
            "channel_id": ch_id,
            "ciphertext": "YWJj",
            "message_type": "text",
            "msg_num": 5
        });
        let msg: WsInbound = serde_json::from_value(json).unwrap();
        match msg {
            WsInbound::SendChannel {
                channel_id,
                ciphertext,
                message_type,
                msg_num,
            } => {
                assert_eq!(channel_id, ch_id);
                assert_eq!(ciphertext, "YWJj");
                assert_eq!(message_type, Some("text".to_string()));
                assert_eq!(msg_num, Some(5));
            }
            other => panic!("Expected SendChannel, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_send_channel_optional_fields_missing() {
        let ch_id = Uuid::new_v4();
        let json = serde_json::json!({
            "type": "send_channel",
            "channel_id": ch_id,
            "ciphertext": "YWJj"
        });
        let msg: WsInbound = serde_json::from_value(json).unwrap();
        match msg {
            WsInbound::SendChannel {
                message_type,
                msg_num,
                ..
            } => {
                assert_eq!(message_type, None);
                assert_eq!(msg_num, None);
            }
            other => panic!("Expected SendChannel, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_typing_start() {
        let ch_id = Uuid::new_v4();
        let json = serde_json::json!({
            "type": "typing_start",
            "channel_id": ch_id
        });
        let msg: WsInbound = serde_json::from_value(json).unwrap();
        match msg {
            WsInbound::TypingStart { channel_id } => assert_eq!(channel_id, ch_id),
            other => panic!("Expected TypingStart, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_read() {
        let msg_id = Uuid::new_v4();
        let ch_id = Uuid::new_v4();
        let json = serde_json::json!({
            "type": "read",
            "message_id": msg_id,
            "channel_id": ch_id
        });
        let msg: WsInbound = serde_json::from_value(json).unwrap();
        match msg {
            WsInbound::Read {
                message_id,
                channel_id,
            } => {
                assert_eq!(message_id, msg_id);
                assert_eq!(channel_id, ch_id);
            }
            other => panic!("Expected Read, got {:?}", other),
        }
    }

    #[test]
    fn test_ws_inbound_unknown_type_fails() {
        let json = r#"{"type":"unknown_type"}"#;
        let result = serde_json::from_str::<WsInbound>(json);
        assert!(result.is_err(), "Unknown type should fail deserialization");
    }

    #[test]
    fn test_ws_inbound_missing_required_field_fails() {
        // send_dm without required ciphertext
        let json = serde_json::json!({
            "type": "send_dm",
            "conversation_id": Uuid::new_v4(),
            "msg_num": 0
        });
        let result = serde_json::from_value::<WsInbound>(json);
        assert!(
            result.is_err(),
            "Missing required field should fail deserialization"
        );
    }

    // ─── WsOutbound serialization tests ──────────────────────────────────────

    #[test]
    fn test_ws_outbound_pong() {
        let msg = WsOutbound::Pong;
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "pong");
        // Pong has no other fields besides "type"
        assert_eq!(json.as_object().unwrap().len(), 1);
    }

    #[test]
    fn test_ws_outbound_ready() {
        let user_id = Uuid::new_v4();
        let msg = WsOutbound::Ready { user_id };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "ready");
        assert_eq!(json["user_id"], user_id.to_string());
    }

    #[test]
    fn test_ws_outbound_presence_online() {
        let user_id = Uuid::new_v4();
        let msg = WsOutbound::Presence {
            user_id,
            online: true,
            away: false,
            last_seen_at: None,
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "presence");
        assert_eq!(json["user_id"], user_id.to_string());
        assert_eq!(json["online"], true);
        assert_eq!(json["away"], false);
        assert!(json["last_seen_at"].is_null());
    }

    #[test]
    fn test_ws_outbound_presence_offline_with_last_seen() {
        let user_id = Uuid::new_v4();
        let last_seen = "2026-03-21T12:00:00Z".to_string();
        let msg = WsOutbound::Presence {
            user_id,
            online: false,
            away: false,
            last_seen_at: Some(last_seen.clone()),
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "presence");
        assert_eq!(json["online"], false);
        assert_eq!(json["last_seen_at"], last_seen);
    }

    #[test]
    fn test_ws_outbound_presence_away() {
        let user_id = Uuid::new_v4();
        let msg = WsOutbound::Presence {
            user_id,
            online: true,
            away: true,
            last_seen_at: None,
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "presence");
        assert_eq!(json["online"], true);
        assert_eq!(json["away"], true);
    }

    #[test]
    fn test_ws_outbound_message() {
        let payload = serde_json::json!({
            "type": "dm",
            "id": Uuid::new_v4(),
            "ciphertext": "encrypted_data"
        });
        let msg = WsOutbound::Message {
            payload: payload.clone(),
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "message");
        assert_eq!(json["payload"], payload);
    }

    #[test]
    fn test_ws_outbound_error() {
        let msg = WsOutbound::Error {
            code: 4029,
            message: "Message rate limit exceeded".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["code"], 4029);
        assert_eq!(json["message"], "Message rate limit exceeded");
    }

    #[test]
    fn test_ws_outbound_re_auth_required() {
        let msg = WsOutbound::ReAuthRequired;
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "re_auth_required");
    }

    #[test]
    fn test_ws_outbound_typing() {
        let ch_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let msg = WsOutbound::Typing {
            channel_id: ch_id,
            user_id,
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "typing");
        assert_eq!(json["channel_id"], ch_id.to_string());
        assert_eq!(json["user_id"], user_id.to_string());
    }

    #[test]
    fn test_ws_outbound_typing_stop() {
        let ch_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let msg = WsOutbound::TypingStop {
            channel_id: ch_id,
            user_id,
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "typing_stop");
        assert_eq!(json["channel_id"], ch_id.to_string());
        assert_eq!(json["user_id"], user_id.to_string());
    }

    #[test]
    fn test_ws_outbound_read_receipt() {
        let msg_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let msg = WsOutbound::ReadReceipt {
            message_id: msg_id,
            user_id,
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "read_receipt");
        assert_eq!(json["message_id"], msg_id.to_string());
        assert_eq!(json["user_id"], user_id.to_string());
    }

    #[test]
    fn test_ws_outbound_canvas_update() {
        let payload = serde_json::json!({"kind": "music_changed", "url": "https://example.com"});
        let msg = WsOutbound::CanvasUpdate {
            payload: payload.clone(),
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "canvas_update");
        assert_eq!(json["payload"], payload);
    }

    #[test]
    fn test_ws_outbound_parent_notification() {
        let payload = serde_json::json!({"alert": "friend_request", "child_id": Uuid::new_v4()});
        let msg = WsOutbound::ParentNotification {
            payload: payload.clone(),
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "parent_notification");
        assert_eq!(json["payload"], payload);
    }

    // ─── WsOutbound round-trip (serialize → deserialize JSON) ────────────────

    #[test]
    fn test_ws_outbound_serializes_to_valid_json_string() {
        // Verify that serializing to a JSON string works for the WebSocket send path
        let msg = WsOutbound::Error {
            code: 4000,
            message: "Invalid message format".to_string(),
        };
        let text = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["code"], 4000);
    }

    // ─── Hub broadcast tests ─────────────────────────────────────────────────

    #[test]
    fn test_hub_broadcast_sends_to_registered_users() {
        let hub = Hub::new();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let user_c = Uuid::new_v4(); // not registered

        let (tx_a, mut rx_a) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx_a, _) = watch::channel(false);
        hub.register(
            user_a,
            None,
            true,
            ConnectionId::new(),
            ConnectionHandle {
                tx: tx_a,
                close_tx: close_tx_a,
            },
        );

        let (tx_b, mut rx_b) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx_b, _) = watch::channel(false);
        hub.register(
            user_b,
            None,
            true,
            ConnectionId::new(),
            ConnectionHandle {
                tx: tx_b,
                close_tx: close_tx_b,
            },
        );

        hub.broadcast(&[user_a, user_b, user_c], WsOutbound::Pong);

        // user_a and user_b should each receive the message
        assert!(rx_a.try_recv().is_ok());
        assert!(rx_b.try_recv().is_ok());
    }

    #[test]
    fn test_hub_broadcast_respects_fanout_limit() {
        let hub = Hub::new();
        let mut user_ids = Vec::new();
        let mut receivers = Vec::new();

        // Register MAX_FANOUT_MEMBERS + 10 users
        for _ in 0..(MAX_FANOUT_MEMBERS as usize + 10) {
            let uid = Uuid::new_v4();
            let (tx, rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
            let (close_tx, _) = watch::channel(false);
            hub.register(
                uid,
                None,
                true,
                ConnectionId::new(),
                ConnectionHandle { tx, close_tx },
            );
            user_ids.push(uid);
            receivers.push(rx);
        }

        hub.broadcast(&user_ids, WsOutbound::Pong);

        // First MAX_FANOUT_MEMBERS should receive
        let mut received = 0;
        for rx in receivers.iter_mut() {
            if rx.try_recv().is_ok() {
                received += 1;
            }
        }
        assert_eq!(
            received, MAX_FANOUT_MEMBERS as usize,
            "broadcast should cap at MAX_FANOUT_MEMBERS"
        );
    }

    // ─── Hub send_to_user tests ──────────────────────────────────────────────

    #[test]
    fn test_hub_send_to_user_delivers_to_all_connections() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx1, _) = watch::channel(false);
        hub.register(
            user_id,
            None,
            true,
            ConnectionId::new(),
            ConnectionHandle {
                tx: tx1,
                close_tx: close_tx1,
            },
        );

        let (tx2, mut rx2) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx2, _) = watch::channel(false);
        hub.register(
            user_id,
            None,
            true,
            ConnectionId::new(),
            ConnectionHandle {
                tx: tx2,
                close_tx: close_tx2,
            },
        );

        hub.send_to_user(&user_id, WsOutbound::Pong);

        assert!(rx1.try_recv().is_ok(), "connection 1 should receive");
        assert!(rx2.try_recv().is_ok(), "connection 2 should receive");
    }

    // ─── Hub device connection tests ─────────────────────────────────────────

    #[test]
    fn test_hub_device_register_and_send() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();

        let (tx, mut rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx, _) = watch::channel(false);
        hub.register(
            user_id,
            Some(device_id),
            true,
            ConnectionId::new(),
            ConnectionHandle { tx, close_tx },
        );

        assert!(hub.is_device_online(&device_id));

        hub.send_to_device(&device_id, WsOutbound::Pong);
        assert!(rx.try_recv().is_ok(), "device should receive message");
    }

    #[test]
    fn test_hub_device_offline_after_unregister() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let conn_id = ConnectionId::new();

        let (tx, _rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx, _) = watch::channel(false);
        hub.register(
            user_id,
            Some(device_id),
            true,
            conn_id.clone(),
            ConnectionHandle { tx, close_tx },
        );
        assert!(hub.is_device_online(&device_id));

        hub.unregister(&user_id, Some(&device_id), &conn_id);
        assert!(!hub.is_device_online(&device_id));
    }

    // ─── Hub away state tests ────────────────────────────────────────────────

    #[test]
    fn test_hub_away_state() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        assert!(!hub.is_away(&user_id));

        hub.away_users.insert(user_id, ());
        assert!(hub.is_away(&user_id));

        hub.away_users.remove(&user_id);
        assert!(!hub.is_away(&user_id));
    }

    // ─── Hub non-trusted route_user_level=false tests ────────────────────────

    #[test]
    fn test_hub_register_non_trusted_device_not_in_user_connections() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        let (close_tx, _) = watch::channel(false);
        // route_user_level=false (pending trust device)
        hub.register(
            user_id,
            Some(device_id),
            false,
            ConnectionId::new(),
            ConnectionHandle { tx, close_tx },
        );

        // Should NOT appear in user-level connections
        assert!(
            !hub.is_online(&user_id),
            "non-trusted device should not count as user online"
        );
        // But SHOULD appear in device connections
        assert!(hub.is_device_online(&device_id));
    }

    // ─── send_ws_error helper test ───────────────────────────────────────────

    #[test]
    fn test_send_ws_error_sends_error_variant() {
        let (tx, mut rx) = mpsc::channel(MAX_OUTBOUND_QUEUE);
        send_ws_error(&tx, 4029, "Message rate limit exceeded");
        let msg = rx.try_recv().unwrap();
        match msg {
            WsOutbound::Error { code, message } => {
                assert_eq!(code, 4029);
                assert_eq!(message, "Message rate limit exceeded");
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    // ─── ConnectionId tests ──────────────────────────────────────────────────

    #[test]
    fn test_connection_id_unique() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2, "each ConnectionId should be unique");
    }

    #[test]
    fn test_connection_id_clone_eq() {
        let id = ConnectionId::new();
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }
}
