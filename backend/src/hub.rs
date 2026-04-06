use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    RateLimiter,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{
    collections::HashSet,
    net::IpAddr,
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::{constants, devices::DeviceTrustState, error::AppResult, AppState};

/// Max server members to fan out a channel message to (Rule 2 / Rule 3).
const MAX_FANOUT_MEMBERS: i64 = 500;
/// Max queued outbound messages per socket before the connection is dropped.
const MAX_OUTBOUND_QUEUE: usize = 256;
/// Re-authenticate shortly before the access token expires.
const WS_REAUTH_WARNING_LEAD: Duration = Duration::from_secs(60);
/// Grace period after `re_auth_required` — if client hasn't reauthed by this
/// deadline, the connection is forcibly closed.
const WS_REAUTH_GRACE: Duration = Duration::from_secs(30);

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
    /// Cached device trust states — avoids a DB query per inbound WS message.
    /// Entries expire after 60 seconds and are invalidated on approve/revoke.
    trust_cache: DashMap<(Uuid, Uuid), (DeviceTrustState, Instant)>,
    /// Cached DM conversation → recipient_id (avoids 2 DB queries per DM send).
    /// Keyed by (conversation_id, sender_id) → recipient_id.
    dm_recipient_cache: DashMap<(Uuid, Uuid), (Uuid, Instant)>,
    /// Cached channel_id → server_id (avoids 1 DB query per channel send).
    channel_server_cache: DashMap<Uuid, (Uuid, Instant)>,
    /// Cached (user_id, server_id) → is_member (avoids 1 DB query per channel send).
    membership_cache: DashMap<(Uuid, Uuid), (bool, Instant)>,
}

impl Default for Hub {
    fn default() -> Self {
        Self {
            connections: DashMap::new(),
            device_connections: DashMap::new(),
            connection_meta: DashMap::new(),
            msg_limiters: DashMap::new(),
            typing_timers: DashMap::new(),
            away_timers: DashMap::new(),
            away_users: DashMap::new(),
            trust_cache: DashMap::new(),
            dm_recipient_cache: DashMap::new(),
            channel_server_cache: DashMap::new(),
            membership_cache: DashMap::new(),
        }
    }
}

/// TTL for cached device trust state entries.
const TRUST_CACHE_TTL: Duration = Duration::from_secs(60);
/// TTL for cached DM recipient and channel membership entries.
const MEMBERSHIP_CACHE_TTL: Duration = Duration::from_secs(300);

impl Hub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a cached trust state. Returns `None` on miss or expiry.
    fn cached_trust_state(&self, user_id: Uuid, device_id: Uuid) -> Option<DeviceTrustState> {
        self.trust_cache
            .get(&(user_id, device_id))
            .filter(|entry| entry.1.elapsed() < TRUST_CACHE_TTL)
            .map(|entry| entry.0.clone())
    }

    /// Insert or update a trust state in the cache.
    fn cache_trust_state(&self, user_id: Uuid, device_id: Uuid, state: DeviceTrustState) {
        self.trust_cache
            .insert((user_id, device_id), (state, Instant::now()));
    }

    /// Invalidate a cached trust state (called on approve/revoke).
    pub fn invalidate_trust_cache(&self, user_id: Uuid, device_id: Uuid) {
        self.trust_cache.remove(&(user_id, device_id));
    }

    /// Look up a cached DM recipient for a conversation + sender pair.
    fn cached_dm_recipient(&self, conversation_id: Uuid, sender_id: Uuid) -> Option<Uuid> {
        self.dm_recipient_cache
            .get(&(conversation_id, sender_id))
            .filter(|e| e.1.elapsed() < MEMBERSHIP_CACHE_TTL)
            .map(|e| e.0)
    }

    /// Cache a DM recipient lookup result.
    fn cache_dm_recipient(&self, conversation_id: Uuid, sender_id: Uuid, recipient_id: Uuid) {
        self.dm_recipient_cache
            .insert((conversation_id, sender_id), (recipient_id, Instant::now()));
    }

    /// Look up a cached channel → server_id mapping.
    fn cached_channel_server(&self, channel_id: Uuid) -> Option<Uuid> {
        self.channel_server_cache
            .get(&channel_id)
            .filter(|e| e.1.elapsed() < MEMBERSHIP_CACHE_TTL)
            .map(|e| e.0)
    }

    /// Cache a channel → server_id mapping.
    fn cache_channel_server(&self, channel_id: Uuid, server_id: Uuid) {
        self.channel_server_cache
            .insert(channel_id, (server_id, Instant::now()));
    }

    /// Look up cached server membership.
    fn cached_membership(&self, user_id: Uuid, server_id: Uuid) -> Option<bool> {
        self.membership_cache
            .get(&(user_id, server_id))
            .filter(|e| e.1.elapsed() < MEMBERSHIP_CACHE_TTL)
            .map(|e| e.0)
    }

    /// Cache a server membership check result.
    fn cache_membership(&self, user_id: Uuid, server_id: Uuid, is_member: bool) {
        self.membership_cache
            .insert((user_id, server_id), (is_member, Instant::now()));
    }

    /// Invalidate membership cache for a user in a server (on join/leave).
    pub fn invalidate_membership(&self, user_id: Uuid, server_id: Uuid) {
        self.membership_cache.remove(&(user_id, server_id));
    }

    /// Evict expired entries from all Hub caches to bound memory growth.
    /// Called periodically from the global GC task.
    pub fn gc_caches(&self) {
        self.trust_cache
            .retain(|_, (_, ts)| ts.elapsed() < TRUST_CACHE_TTL);
        self.dm_recipient_cache
            .retain(|_, (_, ts)| ts.elapsed() < MEMBERSHIP_CACHE_TTL);
        self.channel_server_cache
            .retain(|_, (_, ts)| ts.elapsed() < MEMBERSHIP_CACHE_TTL);
        self.membership_cache
            .retain(|_, (_, ts)| ts.elapsed() < MEMBERSHIP_CACHE_TTL);
    }

    /// Returns true if the user is within their message rate limit (5/sec, burst 20).
    fn check_msg_rate(&self, user_id: &Uuid) -> bool {
        let limiter = self
            .msg_limiters
            .entry(*user_id)
            .or_insert_with(|| {
                // SAFETY: Literal 5 and 20 are non-zero; NonZeroU32::new cannot fail.
                let quota =
                    governor::Quota::per_second(NonZeroU32::new(5).expect("non-zero constant"))
                        .allow_burst(NonZeroU32::new(20).expect("non-zero constant"));
                Arc::new(RateLimiter::direct(quota))
            })
            .clone();
        limiter.check().is_ok()
    }

    /// Register a new connection. Returns `false` if the user already has
    /// `constants::MAX_CONNECTIONS_PER_USER` active connections (caller should close).
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
            if user_conns.len() >= constants::MAX_CONNECTIONS_PER_USER {
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
        let _ = self.try_send_to_user(user_id, msg);
    }

    /// Send a message to all connections of a specific user and report whether
    /// at least one socket accepted the payload.
    pub(crate) fn try_send_to_user(&self, user_id: &Uuid, msg: WsOutbound) -> bool {
        let mut sent = false;
        if let Some(user_conns) = self.connections.get(user_id) {
            let mut stale = Vec::new();
            for entry in user_conns.iter() {
                if entry.value().tx.try_send(msg.clone()).is_ok() {
                    sent = true;
                } else {
                    stale.push(entry.key().clone());
                }
            }
            drop(user_conns);
            for conn_id in stale {
                self.disconnect_connection(&conn_id);
            }
        }
        sent
    }

    pub fn send_to_device(&self, device_id: &Uuid, msg: WsOutbound) {
        let _ = self.try_send_to_device(device_id, msg);
    }

    /// Send a message to all connections of a specific device and report
    /// whether at least one socket accepted the payload.
    pub(crate) fn try_send_to_device(&self, device_id: &Uuid, msg: WsOutbound) -> bool {
        let mut sent = false;
        if let Some(device_conns) = self.device_connections.get(device_id) {
            let mut stale = Vec::new();
            for entry in device_conns.iter() {
                if entry.value().tx.try_send(msg.clone()).is_ok() {
                    sent = true;
                } else {
                    stale.push(entry.key().clone());
                }
            }
            drop(device_conns);
            for conn_id in stale {
                self.disconnect_connection(&conn_id);
            }
        }
        sent
    }

    /// Fan out a message to multiple users (e.g. all members of a channel).
    ///
    /// Sequential loop using non-blocking `try_send` — each iteration is
    /// microseconds with no `.await` and no DashMap guard held across calls.
    /// At the MVP ceiling of 500 members × 5 devices the loop completes in
    /// tens of milliseconds.
    ///
    /// **Scale-up trigger (audit LOW-004):** if p95 server-side channel send
    /// latency exceeds 50 ms, or `MAX_SERVER_MEMBERS_MVP` is raised above 500,
    /// switch to a chunked `tokio::spawn` fan-out (64-member chunks) to
    /// parallelise the `try_send` calls across tasks.
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
///
/// `deny_unknown_fields` prevents clients from injecting unrecognised fields
/// that could bypass future validation or be silently forwarded.
///
/// # E2EE contract
///
/// * `SendDm` / `SendChannel` carry **ciphertext only** — the server relays
///   without decryption and never has access to plaintext message content.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
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
        client_nonce: Option<String>,
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
    /// Pre-serialized JSON string — serialize once per fan-out instead of N times.
    /// Each send still allocates a String from the Arc<str>, but skips serde entirely.
    #[serde(skip)]
    PreSerialized(Arc<str>),
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

/// HIGH-002: Per-IP rate limiter for WebSocket upgrades.
/// 10 upgrades/sec per IP, burst of 20. Prevents connection-flood DoS
/// against the 256MB VM where each unauthenticated connection holds
/// resources for up to 5 seconds (auth timeout).
static WS_UPGRADE_LIMITER: once_cell::sync::Lazy<
    governor::RateLimiter<
        IpAddr,
        governor::state::keyed::DefaultKeyedStateStore<IpAddr>,
        governor::clock::DefaultClock,
    >,
> = once_cell::sync::Lazy::new(|| {
    governor::RateLimiter::keyed(
        governor::Quota::per_second(NonZeroU32::new(10).expect("non-zero constant"))
            .allow_burst(NonZeroU32::new(20).expect("non-zero constant")),
    )
});

/// Evict stale entries from the WS upgrade rate limiter.
/// Called periodically from the global GC task.
pub fn gc_ws_rate_limiter() {
    WS_UPGRADE_LIMITER.retain_recent();
}

/// GET /ws — WebSocket upgrade handler.
///
/// Authentication happens post-upgrade: the first message must be `{ "type": "auth", "token": "..." }`.
/// Tokens are **not** sent in the query string (prevents URL-logged credentials).
///
/// # Security invariants
///
/// * Per-IP upgrade rate limit (10/min, burst 20) to prevent connection exhaustion.
/// * Max frame size (`constants::MAX_WS_FRAME_SIZE = 64 KB`) enforced at the protocol layer.
/// * Per-user connection cap (`constants::MAX_CONNECTIONS_PER_USER = 5`).
/// * Per-user message rate limit (5 msg/sec, burst 20).
/// * Only trusted devices may send `SendDm` / `SendChannel` messages.
pub async fn ws_handler(
    axum::extract::ConnectInfo(peer_addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // HIGH-002: rate-limit WebSocket upgrades per IP
    let ip = crate::auth::handlers::extract_ip(&headers, Some(peer_addr.ip()), &state);
    if WS_UPGRADE_LIMITER.check_key(&ip).is_err() {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    // MED-010: enforce frame size at the WebSocket protocol layer,
    // rejecting oversized frames before they are fully buffered in memory.
    ws.max_message_size(constants::MAX_WS_FRAME_SIZE)
        .max_frame_size(constants::MAX_WS_FRAME_SIZE)
        .on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

// ─── Socket lifecycle ────────────────────────────────────────────────────────

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();
    let conn_id = ConnectionId::new();
    let (tx, rx) = mpsc::channel::<WsOutbound>(MAX_OUTBOUND_QUEUE);
    let (close_tx, close_rx) = watch::channel(false);

    let mut send_task = spawn_ws_send_task(sender, rx, close_rx);

    let mut auth = match wait_for_auth(&mut receiver, &state).await {
        Some(auth) => auth,
        None => {
            send_task.abort();
            return;
        }
    };

    let handle = ConnectionHandle {
        tx: tx.clone(),
        close_tx: close_tx.clone(),
    };
    let is_trusted = auth.trust_state != Some(DeviceTrustState::PendingTrust);
    if !state.hub.register(
        auth.user_id,
        auth.device_id,
        is_trusted,
        conn_id.clone(),
        handle,
    ) {
        send_ws_error(&tx, 4008, "Too many connections");
        send_task.abort();
        return;
    }

    let _ = tx.try_send(WsOutbound::Ready {
        user_id: auth.user_id,
    });
    deliver_on_connect(&auth, is_trusted, &state, &tx).await;

    run_receive_loop(&mut receiver, &mut send_task, &mut auth, &state, &tx).await;

    state
        .hub
        .unregister(&auth.user_id, auth.device_id.as_ref(), &conn_id);
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
                    let text = match msg {
                        WsOutbound::PreSerialized(ref s) => s.to_string(),
                        _ => match serde_json::to_string(&msg) {
                            Ok(t) => t,
                            Err(_) => { tracing::error!("WS serialize error"); continue; }
                        },
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
    auth: &mut WsAuth,
    state: &AppState,
    tx: &ConnTx,
) {
    let mut processed = 0usize;
    loop {
        let now = Instant::now();
        let remaining = auth.auth_expires_at.saturating_duration_since(now);

        // Emit re_auth_required once when nearing expiry, and set a hard grace deadline.
        if !auth.reauth_required && remaining <= WS_REAUTH_WARNING_LEAD {
            let _ = tx.try_send(WsOutbound::ReAuthRequired);
            auth.reauth_required = true;
            auth.reauth_deadline = Some(now + WS_REAUTH_GRACE);
        }

        // Enforce the reauth grace deadline — if the client hasn't reauthed in time, close.
        if let Some(deadline) = auth.reauth_deadline {
            if now >= deadline {
                send_ws_error(tx, 4001, "Re-authentication grace period expired");
                break;
            }
        }

        if auth.auth_expires_at <= now {
            send_ws_error(tx, 4001, "WebSocket session expired");
            break;
        }

        // Sleep until the earliest of: token expiry or reauth grace deadline.
        let sleep_until = match auth.reauth_deadline {
            Some(deadline) => remaining.min(deadline.saturating_duration_since(now)),
            None => remaining,
        };
        let expiry_sleep = tokio::time::sleep(sleep_until);
        tokio::pin!(expiry_sleep);
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if handle_inbound(text, auth, state, tx).await {
                            break;
                        }
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
            _ = &mut expiry_sleep => {
                // The select fires on either token expiry or grace deadline;
                // the next iteration's checks will emit the appropriate error.
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

    let show_last_seen = match sqlx::query(
        "SELECT COALESCE(show_last_seen, TRUE) AS show_last_seen \
         FROM user_privacy_settings \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(row)) => row.try_get::<bool, _>("show_last_seen").unwrap_or(false),
        Ok(None) => true,
        Err(e) => {
            tracing::warn!(user_id = %user_id, "Failed to load last_seen privacy setting: {e}");
            false
        }
    };

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
            .filter_map(|r| {
                r.try_get::<Uuid, _>("user_id")
                    .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
                    .ok()
            })
            .chain(server_rows.iter().filter_map(|r| {
                r.try_get::<Uuid, _>("user_id")
                    .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
                    .ok()
            }))
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
        last_seen_at: redact_last_seen_for_broadcast(show_last_seen, last_seen_at.clone()),
    };

    state.hub.broadcast(&peer_ids, msg);
}

/// Redact a presence timestamp for peer fanout when the author has disabled it.
pub(crate) fn redact_last_seen_for_broadcast(
    show_last_seen: bool,
    last_seen_at: Option<String>,
) -> Option<String> {
    if show_last_seen {
        last_seen_at
    } else {
        None
    }
}

// ─── Auth ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct WsAuth {
    user_id: Uuid,
    device_id: Option<Uuid>,
    account_type: String,
    trust_state: Option<DeviceTrustState>,
    auth_expires_at: Instant,
    reauth_required: bool,
    /// Hard deadline by which the client must complete re-authentication.
    /// Set to `now + WS_REAUTH_GRACE` when `re_auth_required` is emitted.
    reauth_deadline: Option<Instant>,
}

fn ws_auth_expires_at(exp: i64) -> Instant {
    let remaining = exp.saturating_sub(Utc::now().timestamp()).max(0) as u64;
    Instant::now() + Duration::from_secs(remaining)
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

/// # Security invariant — device-bound trust model
///
/// Non-bot WebSocket connections **must** carry a device-bound token
/// (`device_id` present in JWT claims). Legacy non-device tokens are rejected
/// here for non-bot accounts,
/// preventing unauthenticated-device access to the real-time message hub.
///
/// The message loop (`handle_message`) further enforces trust via
/// `live_device_trust_state`, which re-checks the device table on every
/// message and treats `device_id = None` as untrusted for non-bot accounts.
async fn validate_ws_token(token: &str, state: &AppState) -> Option<WsAuth> {
    let claims = crate::auth::service::validate_access_token(token, &state.jwt_keys)
        .ok()?
        .claims;

    // Reject non-bot tokens without a bound device to keep the hub
    // device-authenticated.
    if claims.account_type != "bot" && claims.device_id.is_none() {
        tracing::warn!(
            user_id = %claims.sub,
            "Rejected WS auth: non-bot token without device_id"
        );
        return None;
    }

    let mut trust_state = None;
    if let Some(device_id) = claims.device_id {
        let device = crate::devices::get_device_for_user(claims.sub, device_id, state)
            .await
            .ok()?;
        if device.revoked_at.is_some() || device.trust_state != DeviceTrustState::Trusted {
            return None;
        }
        trust_state = Some(device.trust_state);
    }

    Some(WsAuth {
        user_id: claims.sub,
        device_id: claims.device_id,
        account_type: claims.account_type,
        trust_state,
        auth_expires_at: ws_auth_expires_at(claims.exp),
        reauth_required: false,
        reauth_deadline: None,
    })
}

async fn live_device_trust_state(
    user_id: Uuid,
    device_id: Uuid,
    state: &AppState,
) -> Option<DeviceTrustState> {
    // Fast path: return cached trust state if within TTL
    if let Some(cached) = state.hub.cached_trust_state(user_id, device_id) {
        return Some(cached);
    }

    // Cache miss — query the database
    let device = crate::devices::get_device_for_user(user_id, device_id, state)
        .await
        .ok()?;
    if device.revoked_at.is_some() || device.trust_state == DeviceTrustState::Revoked {
        state
            .hub
            .cache_trust_state(user_id, device_id, DeviceTrustState::Revoked);
        return None;
    }
    state
        .hub
        .cache_trust_state(user_id, device_id, device.trust_state.clone());
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
async fn deliver_offline_envelopes(device_id: &Uuid, state: &AppState, tx: &ConnTx) {
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
               e.msg_num,
               e.ratchet_pub,
               e.previous_chain_len,
               e.crypto_version
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

    let mut delivered_targets: Vec<(Uuid, Uuid)> = Vec::with_capacity(rows.len());
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
        let Ok(sender_device_id) = row.try_get::<Uuid, _>("sender_device_id") else {
            continue;
        };
        let Ok(sender_signal_device_id) = row.try_get::<i32, _>("sender_signal_device_id") else {
            continue;
        };
        let Ok(recipient_device_id) = row.try_get::<Uuid, _>("recipient_device_id") else {
            continue;
        };
        let Ok(cipher) = row.try_get::<Vec<u8>, _>("ciphertext") else {
            continue;
        };
        let ek: Option<Vec<u8>> = row
            .try_get("ek_public")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
            .flatten();
        let opk_id: Option<i32> = row
            .try_get("opk_id")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
            .flatten();
        let msg_num: i32 = row.try_get("msg_num").unwrap_or(0);
        let ratchet_pub: Option<Vec<u8>> = row
            .try_get("ratchet_pub")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
            .flatten();
        let previous_chain_len: Option<i32> = row
            .try_get("previous_chain_len")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
            .flatten();
        let crypto_version: i32 = row.try_get("crypto_version").unwrap_or(1);

        let payload = dm_v2_payload(
            msg_id,
            conv_id,
            sender_id,
            sender_device_id,
            sender_signal_device_id,
            recipient_device_id,
            &BASE64.encode(&cipher),
            ek.as_ref().map(|k| BASE64.encode(k)),
            opk_id,
            msg_num,
            ratchet_pub.as_ref().map(|k| BASE64.encode(k)),
            previous_chain_len,
            crypto_version,
            row.try_get("created_at").ok(),
        );

        if tx.try_send(WsOutbound::Message { payload }).is_ok() {
            delivered_targets.push((msg_id, recipient_device_id));
        } else {
            tracing::debug!("Recipient disconnected during DM v2 replay");
            break;
        }
    }

    if !delivered_targets.is_empty() {
        if let Err(e) = mark_dm_delivered(&delivered_targets, state).await {
            tracing::warn!("Failed to mark DM envelopes delivered: {e}");
        }
    }
}

/// Deliver legacy DM messages (pre-multi-device path, no device_id).
async fn deliver_offline_legacy(user_id: &Uuid, state: &AppState, tx: &ConnTx) {
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
        let ek: Option<Vec<u8>> = row
            .try_get("ek_public")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
            .flatten();
        let opk_id: Option<i32> = row
            .try_get("opk_id")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
            .flatten();

        let payload = serde_json::json!({
            "type": "dm", "id": msg_id, "conversation_id": conv_id,
            "sender_id": sender_id, "ciphertext": BASE64.encode(&cipher),
            "ephemeral_key": ek.as_ref().map(|k| BASE64.encode(k)), "opk_id": opk_id,
        });

        if tx.try_send(WsOutbound::Message { payload }).is_ok() {
            delivered_ids.push(msg_id);
        } else {
            tracing::debug!("Recipient disconnected during legacy DM replay");
            break;
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
            "SELECT id, channel_id, from_user, from_device_id, ciphertext, ek_public \
             FROM sender_key_distributions \
             WHERE delivered = FALSE \
               AND (to_device_id = $1 OR (to_device_id IS NULL AND to_user = $2)) \
             ORDER BY created_at ASC \
             LIMIT 100",
        )
        .bind(device_id)
        .bind(user_id)
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT id, channel_id, from_user, from_device_id, ciphertext, ek_public \
             FROM sender_key_distributions \
             WHERE to_user = $1 AND delivered = FALSE \
             ORDER BY created_at ASC \
             LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(state.db.pool())
        .await
    };

    let Ok(rows) = rows else { return };

    let mut delivered_ids: Vec<Uuid> = Vec::with_capacity(rows.len());
    for row in &rows {
        let Ok(id) = row.try_get::<Uuid, _>("id") else {
            continue;
        };
        let Ok(channel_id) = row.try_get::<uuid::Uuid, _>("channel_id") else {
            continue;
        };
        let Ok(from_user) = row.try_get::<uuid::Uuid, _>("from_user") else {
            continue;
        };
        let from_device_id: Option<uuid::Uuid> = row
            .try_get("from_device_id")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
            .flatten();
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
        if tx.try_send(WsOutbound::Message { payload }).is_ok() {
            delivered_ids.push(id);
        } else {
            tracing::debug!("Recipient disconnected during key dist delivery");
            break;
        }
    }

    if !delivered_ids.is_empty() {
        if let Err(e) = sqlx::query(
            "UPDATE sender_key_distributions \
             SET delivered = TRUE \
             WHERE id = ANY($1) AND delivered = FALSE",
        )
        .bind(&delivered_ids)
        .execute(state.db.pool())
        .await
        {
            tracing::warn!("Failed to mark sender key distributions delivered: {e}");
        }
    }
}

async fn deliver_pending_sync_events(device_id: Option<&Uuid>, state: &AppState, tx: &ConnTx) {
    let Some(device_id) = device_id else {
        return;
    };

    let rows = sqlx::query(
        r#"
        SELECT id, event_type, source_device_id, payload, created_at
        FROM device_sync_events
        WHERE target_device_id = $1 AND delivered_at IS NULL
        ORDER BY created_at ASC
        LIMIT 100
        "#,
    )
    .bind(device_id)
    .fetch_all(state.db.pool())
    .await;

    let Ok(rows) = rows else {
        return;
    };
    if rows.is_empty() {
        return;
    }

    for row in &rows {
        let Ok(event) = row.try_get::<Uuid, _>("id") else {
            continue;
        };
        let Ok(sync_event) = row.try_get::<String, _>("event_type") else {
            continue;
        };
        let payload_value: serde_json::Value = match row.try_get("payload") {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!("Column extraction failed: {e}");
                continue;
            }
        };
        let event = crate::devices::SyncEvent {
            id: event,
            event_type: sync_event,
            source_device_id: row.try_get("source_device_id").ok().flatten(),
            payload: payload_value,
            created_at: row
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        };

        let payload = crate::devices::sync_event_payload(&event);
        if tx.try_send(WsOutbound::Message { payload }).is_err() {
            tracing::debug!("Device disconnected during sync event delivery");
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn dm_v2_payload(
    message_id: Uuid,
    conv_id: Uuid,
    sender_id: Uuid,
    sender_device_id: Uuid,
    sender_signal_device_id: i32,
    recipient_device_id: Uuid,
    ciphertext: &str,
    ephemeral_key: Option<String>,
    opk_id: Option<i32>,
    msg_num: i32,
    ratchet_pub: Option<String>,
    previous_chain_len: Option<i32>,
    crypto_version: i32,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "type": "dm_v2",
        "id": message_id,
        "conversation_id": conv_id,
        "sender_id": sender_id,
        "sender_device_id": sender_device_id,
        "sender_signal_device_id": sender_signal_device_id,
        "recipient_device_id": recipient_device_id,
        "ciphertext": ciphertext,
        "ephemeral_key": ephemeral_key,
        "opk_id": opk_id,
        "msg_num": msg_num,
        "ratchet_pub": ratchet_pub,
        "previous_chain_len": previous_chain_len,
        "crypto_version": crypto_version,
    });
    if let Some(created_at) = created_at {
        payload["created_at"] = serde_json::json!(created_at);
    }
    payload
}

pub(crate) async fn mark_dm_delivered(
    delivery_targets: &[(Uuid, Uuid)],
    state: &AppState,
) -> AppResult<()> {
    if delivery_targets.is_empty() {
        return Ok(());
    }

    let mut tx = state.db.pool().begin().await?;

    let mut delivered_messages = HashSet::with_capacity(delivery_targets.len());
    for &(message_id, recipient_device_id) in delivery_targets {
        sqlx::query(
            "UPDATE dm_message_envelopes \
             SET delivered_at = NOW() \
             WHERE message_id = $1 \
               AND recipient_device_id = $2 \
               AND delivered_at IS NULL",
        )
        .bind(message_id)
        .bind(recipient_device_id)
        .execute(&mut *tx)
        .await?;
        delivered_messages.insert(message_id);
    }

    let delivered_message_ids: Vec<Uuid> = delivered_messages.into_iter().collect();
    if !delivered_message_ids.is_empty() {
        sqlx::query(
            "UPDATE messages SET delivered = TRUE WHERE id = ANY($1) AND delivered = FALSE",
        )
        .bind(&delivered_message_ids)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

// ─── Inbound dispatch ────────────────────────────────────────────────────────

async fn handle_inbound(text: String, auth: &mut WsAuth, state: &AppState, tx: &ConnTx) -> bool {
    if text.len() > constants::MAX_WS_FRAME_SIZE {
        send_ws_error(tx, 4003, "Frame too large");
        return auth.reauth_required;
    }

    let msg: WsInbound = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(_) => {
            send_ws_error(tx, 4000, "Invalid message format");
            return auth.reauth_required;
        }
    };

    let is_control_message = matches!(
        &msg,
        WsInbound::Ping | WsInbound::Auth { .. } | WsInbound::Reauth { .. }
    );

    if auth.reauth_required && !matches!(&msg, WsInbound::Ping | WsInbound::Reauth { .. }) {
        send_ws_error(tx, 4001, "Re-authentication required");
        return true;
    }

    let user_id = auth.user_id;
    let device_id = auth.device_id;

    // Device trust gate: only trusted devices (or bots) may send real messages.
    // `device_id = None` is only reachable for bot accounts — `validate_ws_token`
    // already rejected non-bot tokens without a device_id at connection time.
    let device_is_trusted = match auth.device_id {
        Some(device_id) => match live_device_trust_state(auth.user_id, device_id, state).await {
            Some(DeviceTrustState::Trusted) => true,
            Some(DeviceTrustState::PendingTrust) => false,
            Some(DeviceTrustState::Revoked) | None => {
                send_ws_error(tx, 4001, "Device revoked");
                return true;
            }
        },
        // Only bots reach here (non-bot device-less tokens are rejected in
        // `validate_ws_token`). Bots are trusted by definition.
        None => {
            debug_assert_eq!(
                auth.account_type, "bot",
                "non-bot token without device_id should have been rejected at WS auth"
            );
            auth.account_type == "bot"
        }
    };

    if !device_is_trusted && !is_control_message {
        send_ws_error(tx, 4006, "Device approval required");
        return false;
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
            client_nonce,
        } => {
            if check_rate_limit(state, &user_id, tx) {
                handle_send_channel(
                    channel_id,
                    ciphertext,
                    message_type,
                    msg_num,
                    client_nonce,
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
            let Some(next_auth) = validate_ws_reauth(&token, auth, state).await else {
                send_ws_error(tx, 4001, "Invalid token");
                return true;
            };
            *auth = next_auth;
            auth.reauth_required = false;
            auth.reauth_deadline = None;
        }
        WsInbound::Auth { .. } => {}
    }

    false
}

async fn validate_ws_reauth(token: &str, current: &WsAuth, state: &AppState) -> Option<WsAuth> {
    let next = validate_ws_token(token, state).await?;
    if next.user_id != current.user_id
        || next.device_id != current.device_id
        || next.account_type != current.account_type
    {
        return None;
    }
    Some(next)
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

    // WebSocket DM sends are capped on ciphertext wire bytes; plaintext length is a client concern.
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
    // Fast path: return cached recipient if within TTL
    if let Some(cached) = state.hub.cached_dm_recipient(conversation_id, sender_id) {
        return Some(cached);
    }

    let is_participant =
        sqlx::query("SELECT 1 FROM dm_participants WHERE conversation_id = $1 AND user_id = $2")
            .bind(conversation_id)
            .bind(sender_id)
            .fetch_optional(state.db.pool())
            .await
            .map_err(|e| tracing::error!("DB query failed: {e}"))
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
        Ok(Some(row)) => match row
            .try_get("user_id")
            .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
            .ok()
        {
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
        .map_err(|e| tracing::error!("DB query failed: {e}"))
        .ok()
        .flatten()
        .and_then(|r| {
            r.try_get::<String, _>("account_type")
                .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
                .ok()
        })
        .map(|t| t == "bot")
        .unwrap_or(false);

    if is_bot {
        send_ws_error(tx, 4011, "Cannot send DMs to bot accounts");
        return None;
    }

    state
        .hub
        .cache_dm_recipient(conversation_id, sender_id, recipient_id);
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
    .bind(false)
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
    let delivered = state
        .hub
        .try_send_to_user(&recipient_id, WsOutbound::Message { payload });

    if delivered {
        if let Err(e) =
            sqlx::query("UPDATE messages SET delivered = TRUE WHERE id = $1 AND delivered = FALSE")
                .bind(msg_id)
                .execute(state.db.pool())
                .await
        {
            tracing::warn!("Failed to mark DM delivered: {e}");
        }
    }

    // Push notification to offline devices (best-effort, fire-and-forget)
    if !delivered {
        let state = state.clone();
        tokio::spawn(async move {
            let mut meta = std::collections::HashMap::new();
            meta.insert("conversation_id".into(), conversation_id.to_string());
            meta.insert("sender_id".into(), sender_id.to_string());
            crate::notifications::notify_user_offline_devices(recipient_id, "dm", &meta, &state)
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
    client_nonce: Option<String>,
    sender_id: Uuid,
    sender_device_id: Option<Uuid>,
    state: &AppState,
    tx: &ConnTx,
) {
    debug_assert!(sender_id != Uuid::nil());

    // WebSocket channel sends are validated on ciphertext size after client-side encryption.
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
            channel_id,
            server_id,
            &ciphertext,
            msg_type,
            sender_id,
            state,
            tx,
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
            channel_id,
            server_id,
            &cipher_bytes,
            msg_type,
            msg_num,
            client_nonce.as_deref(),
            &ciphertext,
            sender_id,
            sender_device_id,
            state,
            tx,
        )
        .await;
    }
}

async fn is_bot_user(user_id: Uuid, state: &AppState) -> bool {
    sqlx::query("SELECT account_type FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(|e| tracing::error!("DB query failed: {e}"))
        .ok()
        .flatten()
        .and_then(|r| {
            r.try_get::<String, _>("account_type")
                .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
                .ok()
        })
        .map(|t| t == "bot")
        .unwrap_or(false)
}

async fn resolve_channel_membership(
    channel_id: Uuid,
    sender_id: Uuid,
    state: &AppState,
    tx: &ConnTx,
) -> Option<Uuid> {
    // Fast path: look up channel → server_id from cache
    let server_id = if let Some(cached) = state.hub.cached_channel_server(channel_id) {
        cached
    } else {
        let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(state.db.pool())
            .await
            .map_err(|e| tracing::error!("DB query failed: {e}"))
            .ok()
            .flatten();

        match server_row.and_then(|r| {
            r.try_get("server_id")
                .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
                .ok()
        }) {
            Some(id) => {
                state.hub.cache_channel_server(channel_id, id);
                id
            }
            None => {
                send_ws_error(tx, 4006, "Channel not found");
                return None;
            }
        }
    };

    // Fast path: look up membership from cache
    let is_member = if let Some(cached) = state.hub.cached_membership(sender_id, server_id) {
        cached
    } else {
        let found =
            sqlx::query("SELECT 1 FROM server_memberships WHERE user_id = $1 AND server_id = $2")
                .bind(sender_id)
                .bind(server_id)
                .fetch_optional(state.db.pool())
                .await
                .map_err(|e| tracing::error!("DB query failed: {e}"))
                .ok()
                .flatten()
                .is_some();
        state.hub.cache_membership(sender_id, server_id, found);
        found
    };

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
    client_nonce: Option<&str>,
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
            "client_nonce": client_nonce,
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

    // Pre-serialize once for all recipients instead of serializing per-connection.
    let wrapper = WsOutbound::Message { payload };
    let json: Arc<str> = match serde_json::to_string(&wrapper) {
        Ok(s) => Arc::from(s),
        Err(e) => {
            tracing::error!("Failed to serialize fanout message: {e}");
            return;
        }
    };

    for m in member_rows.iter().take(MAX_FANOUT_MEMBERS as usize) {
        if let Ok(uid) = m.try_get::<Uuid, _>("user_id") {
            state
                .hub
                .send_to_user(&uid, WsOutbound::PreSerialized(Arc::clone(&json)));
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
        .map_err(|e| tracing::error!("DB query failed: {e}"))
        .ok()
        .flatten()?;
    let server_id: Uuid = server_row
        .try_get("server_id")
        .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
        .ok()?;

    let rows = sqlx::query("SELECT user_id FROM server_memberships WHERE server_id = $1 LIMIT $2")
        .bind(server_id)
        .bind(MAX_FANOUT_MEMBERS)
        .fetch_all(state.db.pool())
        .await
        .map_err(|e| tracing::error!("DB query failed: {e}"))
        .ok()?;

    Some(
        rows.iter()
            .filter_map(|r| {
                r.try_get::<Uuid, _>("user_id")
                    .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
                    .ok()
            })
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

    // Verify the message belongs to the caller-supplied channel before processing read receipt.
    let Some(member_ids) =
        load_read_receipt_member_ids(message_id, channel_id, user_id, state).await
    else {
        return;
    };

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

/// Resolve the channel members eligible to receive a read receipt fanout.
///
/// Returns `None` when the message is missing, deleted, not a channel message,
/// mapped to a different channel than the caller supplied, or when the caller
/// is not a member of the real channel.
pub async fn load_read_receipt_member_ids(
    message_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> Option<Vec<Uuid>> {
    debug_assert!(message_id != Uuid::nil());
    debug_assert!(channel_id != Uuid::nil());
    debug_assert!(user_id != Uuid::nil());

    let message_row = sqlx::query(
        "SELECT channel_id \
         FROM messages \
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(message_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| tracing::error!("DB query failed: {e}"))
    .ok()
    .flatten()?;

    let real_channel_id = message_row
        .try_get::<Option<Uuid>, _>("channel_id")
        .map_err(|e| tracing::warn!("Column extraction failed: {e}"))
        .ok()
        .flatten()?;
    if real_channel_id != channel_id {
        return None;
    }

    let member_ids = fetch_channel_member_ids(real_channel_id, state).await?;
    if !member_ids.contains(&user_id) {
        return None;
    }

    Some(member_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{service::generate_access_token, JwtKeys, LoginRateLimiter, OAuthStateStore},
        db::Database,
        DiscordImportStateStore, IpRateLimiter,
    };
    use governor::{Quota, RateLimiter};
    use sqlx::PgPool;
    use std::{collections::HashSet, num::NonZeroU32, sync::Arc};
    use tokio::sync::{mpsc, watch};

    fn build_test_state_from_pool(pool: PgPool) -> Option<AppState> {
        let jwt_keys = match JwtKeys::from_env() {
            Ok(keys) => Arc::new(keys),
            Err(_) => return None,
        };

        let quota = Quota::per_minute(NonZeroU32::new(10_000).unwrap())
            .allow_burst(NonZeroU32::new(5_000).unwrap());
        let rate_limiter: IpRateLimiter = Arc::new(RateLimiter::keyed(quota));

        Some(AppState {
            db: Database::from_pool(pool),
            hub: Arc::new(Hub::new()),
            rate_limiter,
            trusted_proxy_ips: Arc::new(HashSet::new()),
            jwt_keys,
            login_limiter: Arc::new(LoginRateLimiter::new()),
            oauth_states: Arc::new(OAuthStateStore::new()),
            discord_import_states: Arc::new(DiscordImportStateStore::new()),
            http_client: reqwest::Client::new(),
        })
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn ws_auth_rejects_pending_trust_device(pool: PgPool) {
        let Some(state) = build_test_state_from_pool(pool) else {
            return;
        };

        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, display_name, gdpr_consent_at)
             VALUES ($1, $2, $3, $4, NOW())",
        )
        .bind(user_id)
        .bind("ws_pending_user")
        .bind(format!("{user_id}@integration.test"))
        .bind("WS Pending User")
        .execute(state.db.pool())
        .await
        .expect("insert user");

        sqlx::query(
            "INSERT INTO devices
                (id, user_id, signal_device_id, platform, label, trust_state, last_seen_at, approved_at)
             VALUES ($1, $2, $3, $4, $5, 'pending_trust', NOW(), NULL)",
        )
        .bind(device_id)
        .bind(user_id)
        .bind(1_i32)
        .bind("web")
        .bind("Pending WS Device")
        .execute(state.db.pool())
        .await
        .expect("insert device");

        let token =
            generate_access_token(user_id, "standard", Some(device_id), &state.jwt_keys).unwrap();

        assert!(
            validate_ws_token(&token, &state).await.is_none(),
            "pending-trust devices must not authenticate to WS"
        );
    }

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
        for _ in 0..constants::MAX_CONNECTIONS_PER_USER {
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

    #[test]
    fn test_hub_try_send_to_user_returns_false_when_queue_full() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        let conn_id = ConnectionId::new();
        let (tx, _rx) = mpsc::channel(1);
        let (close_tx, _close_rx) = watch::channel(false);
        assert!(hub.register(
            user_id,
            None,
            true,
            conn_id,
            ConnectionHandle {
                tx: tx.clone(),
                close_tx,
            },
        ));
        tx.try_send(WsOutbound::Pong).unwrap();

        assert!(
            !hub.try_send_to_user(&user_id, WsOutbound::Pong),
            "queue-full user send should report failure"
        );
        assert!(
            !hub.is_online(&user_id),
            "stale user connection should be disconnected after send failure"
        );
    }

    #[test]
    fn test_hub_try_send_to_device_returns_false_when_queue_full() {
        let hub = Hub::new();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let conn_id = ConnectionId::new();
        let (tx, _rx) = mpsc::channel(1);
        let (close_tx, _close_rx) = watch::channel(false);
        assert!(hub.register(
            user_id,
            Some(device_id),
            true,
            conn_id,
            ConnectionHandle {
                tx: tx.clone(),
                close_tx,
            },
        ));
        tx.try_send(WsOutbound::Pong).unwrap();

        assert!(
            !hub.try_send_to_device(&device_id, WsOutbound::Pong),
            "queue-full device send should report failure"
        );
        assert!(
            !hub.is_device_online(&device_id),
            "stale device connection should be disconnected after send failure"
        );
    }

    #[test]
    fn test_dm_v2_replay_payload_includes_ratchet_metadata() {
        let payload = dm_v2_payload(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            7,
            Uuid::new_v4(),
            "AQID",
            Some("BAUG".to_string()),
            Some(42),
            9,
            Some("BwgJ".to_string()),
            Some(11),
            2,
            None,
        );

        assert_eq!(payload["type"], "dm_v2");
        assert_eq!(payload["ciphertext"], "AQID");
        assert_eq!(payload["ratchet_pub"], "BwgJ");
        assert_eq!(payload["previous_chain_len"], 11);
        assert_eq!(payload["crypto_version"], 2);
    }

    // ─── Rate limiter tests ──────────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn test_mark_dm_delivered_advances_state_after_success(pool: PgPool) {
        let Some(state) = build_test_state_from_pool(pool) else {
            return;
        };

        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let sender_device = Uuid::new_v4();
        let recipient_device = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO users (id, email, username, display_name, password_hash, gdpr_consent_at) \
             VALUES ($1, $2, $3, $4, $5, NOW()), ($6, $7, $8, $9, $10, NOW())",
        )
        .bind(user_a)
        .bind(format!("hub_a_{message_id}@integration.test"))
        .bind(format!("hub_a_{message_id}"))
        .bind("Hub A")
        .bind("hash")
        .bind(user_b)
        .bind(format!("hub_b_{message_id}@integration.test"))
        .bind(format!("hub_b_{message_id}"))
        .bind("Hub B")
        .bind("hash")
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO devices (id, user_id, signal_device_id, platform, label, trust_state, last_seen_at, approved_at) \
             VALUES ($1, $2, 1001, 'web', 'sender', 'trusted', NOW(), NOW()), \
                    ($3, $4, 1002, 'web', 'recipient', 'trusted', NOW(), NOW())",
        )
        .bind(sender_device)
        .bind(user_a)
        .bind(recipient_device)
        .bind(user_b)
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query("INSERT INTO dm_conversations (id, created_at) VALUES ($1, NOW())")
            .bind(conversation_id)
            .execute(state.db.pool())
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO dm_participants (conversation_id, user_id) VALUES ($1, $2), ($1, $3)",
        )
        .bind(conversation_id)
        .bind(user_a)
        .bind(user_b)
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, conversation_id, sender_id, sender_device_id, ciphertext, delivered, created_at) \
             VALUES ($1, $2, $3, $4, $5, FALSE, NOW())",
        )
        .bind(message_id)
        .bind(conversation_id)
        .bind(user_a)
        .bind(sender_device)
        .bind(vec![1_u8, 2, 3])
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO dm_message_envelopes \
             (message_id, recipient_user_id, recipient_device_id, sender_user_id, sender_device_id, \
              ciphertext, ek_public, opk_id, msg_num, ratchet_pub, previous_chain_len, crypto_version, delivered_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NULL, NULL, 1, NULL, NULL, 1, NULL, NOW())",
        )
        .bind(message_id)
        .bind(user_b)
        .bind(recipient_device)
        .bind(user_a)
        .bind(sender_device)
        .bind(vec![4_u8, 5, 6])
        .execute(state.db.pool())
        .await
        .unwrap();

        let undelivered_before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND delivered = FALSE",
        )
        .bind(conversation_id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
        assert_eq!(undelivered_before, 1, "message should start as undelivered");

        mark_dm_delivered(&[(message_id, recipient_device)], &state)
            .await
            .unwrap();

        let msg_delivered: Option<bool> =
            sqlx::query_scalar("SELECT delivered FROM messages WHERE id = $1")
                .bind(message_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert_eq!(msg_delivered, Some(true));

        let envelope_delivered_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar(
                "SELECT delivered_at FROM dm_message_envelopes WHERE message_id = $1 AND recipient_device_id = $2",
            )
            .bind(message_id)
            .bind(recipient_device)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
        assert!(
            envelope_delivered_at.is_some(),
            "envelope should be marked delivered"
        );

        let undelivered_after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND delivered = FALSE",
        )
        .bind(conversation_id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
        assert_eq!(
            undelivered_after, 0,
            "mark_dm_delivered should transition the state"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pending_sync_events_keep_state_on_queue_full(pool: PgPool) {
        let Some(state) = build_test_state_from_pool(pool) else {
            return;
        };

        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let conn_id = ConnectionId::new();
        let (tx, _rx) = mpsc::channel(1);
        let (close_tx, _close_rx) = watch::channel(false);
        assert!(state.hub.register(
            user_id,
            Some(device_id),
            true,
            conn_id,
            ConnectionHandle {
                tx: tx.clone(),
                close_tx,
            },
        ));
        tx.try_send(WsOutbound::Pong).unwrap();

        sqlx::query(
            "INSERT INTO users (id, email, username, display_name, password_hash, gdpr_consent_at) \
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(user_id)
        .bind(format!("sync_{user_id}@integration.test"))
        .bind(format!("sync_{user_id}"))
        .bind("Sync User")
        .bind("hash")
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO devices (id, user_id, signal_device_id, platform, label, trust_state, last_seen_at, approved_at) \
             VALUES ($1, $2, 2001, 'web', 'sync-device', 'trusted', NOW(), NOW())",
        )
        .bind(device_id)
        .bind(user_id)
        .execute(state.db.pool())
        .await
        .unwrap();

        let event_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO device_sync_events \
             (id, user_id, target_device_id, source_device_id, event_type, payload, delivered_at, created_at) \
             VALUES ($1, $2, $3, NULL, 'device_sync_complete', '{}'::jsonb, NULL, NOW())",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(device_id)
        .execute(state.db.pool())
        .await
        .unwrap();

        deliver_pending_sync_events(Some(&device_id), &state, &tx).await;

        let delivered_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT delivered_at FROM device_sync_events WHERE id = $1")
                .bind(event_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert!(
            delivered_at.is_none(),
            "sync event should remain pending on queue failure"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pending_sync_events_require_ack_after_successful_ws_handoff(pool: PgPool) {
        let Some(state) = build_test_state_from_pool(pool) else {
            return;
        };

        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let conn_id = ConnectionId::new();
        let (tx, mut rx) = mpsc::channel(2);
        let (close_tx, _close_rx) = watch::channel(false);
        assert!(state.hub.register(
            user_id,
            Some(device_id),
            true,
            conn_id,
            ConnectionHandle {
                tx: tx.clone(),
                close_tx
            },
        ));

        sqlx::query(
            "INSERT INTO users (id, email, username, display_name, password_hash, gdpr_consent_at) \
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(user_id)
        .bind(format!("sync_ack_{user_id}@integration.test"))
        .bind(format!("sync_ack_{user_id}"))
        .bind("Sync Ack User")
        .bind("hash")
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO devices (id, user_id, signal_device_id, platform, label, trust_state, last_seen_at, approved_at) \
             VALUES ($1, $2, 2002, 'web', 'sync-device', 'trusted', NOW(), NOW())",
        )
        .bind(device_id)
        .bind(user_id)
        .execute(state.db.pool())
        .await
        .unwrap();

        let event_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO device_sync_events \
             (id, user_id, target_device_id, source_device_id, event_type, payload, delivered_at, created_at) \
             VALUES ($1, $2, $3, NULL, 'device_sync_complete', '{}'::jsonb, NULL, NOW())",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(device_id)
        .execute(state.db.pool())
        .await
        .unwrap();

        deliver_pending_sync_events(Some(&device_id), &state, &tx).await;

        let next = rx.recv().await.expect("sync event should be enqueued");
        match next {
            WsOutbound::Message { payload } => {
                let expected_id = event_id.to_string();
                assert_eq!(payload["type"], "device_sync_event");
                assert_eq!(payload["id"].as_str(), Some(expected_id.as_str()));
            }
            other => panic!("unexpected ws frame: {other:?}"),
        }

        let delivered_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT delivered_at FROM device_sync_events WHERE id = $1")
                .bind(event_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert!(
            delivered_at.is_none(),
            "ws handoff alone must not mark sync events delivered"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pending_key_dists_keep_state_on_queue_full(pool: PgPool) {
        let Some(state) = build_test_state_from_pool(pool) else {
            return;
        };

        let sender_user = Uuid::new_v4();
        let recipient_user = Uuid::new_v4();
        let recipient_device = Uuid::new_v4();
        let server_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(1);
        let (close_tx, _close_rx) = watch::channel(false);
        let conn_id = ConnectionId::new();

        assert!(state.hub.register(
            recipient_user,
            Some(recipient_device),
            true,
            conn_id,
            ConnectionHandle {
                tx: tx.clone(),
                close_tx,
            },
        ));
        tx.try_send(WsOutbound::Pong).unwrap();

        sqlx::query(
            "INSERT INTO users (id, email, username, display_name, password_hash, gdpr_consent_at) \
             VALUES ($1, $2, $3, $4, $5, NOW()), ($6, $7, $8, $9, $10, NOW())",
        )
        .bind(sender_user)
        .bind(format!("skd_sender_{channel_id}@integration.test"))
        .bind(format!("skd_sender_{channel_id}"))
        .bind("SKD Sender")
        .bind("hash")
        .bind(recipient_user)
        .bind(format!("skd_recipient_{channel_id}@integration.test"))
        .bind(format!("skd_recipient_{channel_id}"))
        .bind("SKD Recipient")
        .bind("hash")
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO devices (id, user_id, signal_device_id, platform, label, trust_state, last_seen_at, approved_at) \
             VALUES ($1, $2, 3001, 'web', 'recipient-device', 'trusted', NOW(), NOW())",
        )
        .bind(recipient_device)
        .bind(recipient_user)
        .execute(state.db.pool())
        .await
        .unwrap();

        sqlx::query("INSERT INTO servers (id, name, slug, owner_id, created_at) VALUES ($1, $2, $3, $4, NOW())")
            .bind(server_id)
            .bind("SKD Server")
            .bind(format!("skd-{channel_id}"))
            .bind(sender_user)
            .execute(state.db.pool())
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO channels (id, server_id, name, type, position, created_at) \
             VALUES ($1, $2, $3, 'text', 0, NOW())",
        )
        .bind(channel_id)
        .bind(server_id)
        .bind("general")
        .execute(state.db.pool())
        .await
        .unwrap();

        let dist_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO sender_key_distributions \
             (id, channel_id, from_user, from_device_id, to_user, to_device_id, ciphertext, ek_public, delivered, created_at) \
             VALUES ($1, $2, $3, NULL, $4, $5, $6, $7, FALSE, NOW())",
        )
        .bind(dist_id)
        .bind(channel_id)
        .bind(sender_user)
        .bind(recipient_user)
        .bind(recipient_device)
        .bind(vec![9_u8, 8, 7])
        .bind(vec![6_u8, 5, 4])
        .execute(state.db.pool())
        .await
        .unwrap();

        deliver_pending_key_dists(&recipient_user, Some(&recipient_device), &state, &tx).await;

        let delivered: bool =
            sqlx::query_scalar("SELECT delivered FROM sender_key_distributions WHERE id = $1")
                .bind(dist_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert!(
            !delivered,
            "sender-key dist should remain pending on queue failure"
        );
    }

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
            "msg_num": 5,
            "client_nonce": "abc-123"
        });
        let msg: WsInbound = serde_json::from_value(json).unwrap();
        match msg {
            WsInbound::SendChannel {
                channel_id,
                ciphertext,
                message_type,
                msg_num,
                client_nonce,
            } => {
                assert_eq!(channel_id, ch_id);
                assert_eq!(ciphertext, "YWJj");
                assert_eq!(message_type, Some("text".to_string()));
                assert_eq!(msg_num, Some(5));
                assert_eq!(client_nonce, Some("abc-123".to_string()));
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
                client_nonce,
                ..
            } => {
                assert_eq!(message_type, None);
                assert_eq!(msg_num, None);
                assert_eq!(client_nonce, None);
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
    fn test_redact_last_seen_for_broadcast_hides_when_disabled() {
        let last_seen = Some("2026-03-21T12:00:00Z".to_string());
        assert_eq!(redact_last_seen_for_broadcast(false, last_seen), None);
    }

    #[test]
    fn test_redact_last_seen_for_broadcast_keeps_when_enabled() {
        let last_seen = Some("2026-03-21T12:00:00Z".to_string());
        assert_eq!(
            redact_last_seen_for_broadcast(true, last_seen.clone()),
            last_seen
        );
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
