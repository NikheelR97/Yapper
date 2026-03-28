use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthDevice,
    error::{AppError, AppResult},
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTrustState {
    Trusted,
    PendingTrust,
    Revoked,
}

impl DeviceTrustState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::PendingTrust => "pending_trust",
            Self::Revoked => "revoked",
        }
    }
}

impl TryFrom<&str> for DeviceTrustState {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "trusted" => Ok(Self::Trusted),
            "pending_trust" => Ok(Self::PendingTrust),
            "revoked" => Ok(Self::Revoked),
            _ => Err(AppError::Internal(anyhow::anyhow!(
                "Invalid device trust state {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    pub id: Uuid,
    pub signal_device_id: i32,
    pub installation_id: Option<Uuid>,
    pub platform: String,
    pub label: String,
    pub trust_state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceBootstrap {
    pub installation_id: Option<Uuid>,
    pub platform: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub id: Uuid,
    pub signal_device_id: i32,
    pub installation_id: Option<Uuid>,
    pub platform: String,
    pub label: String,
    pub trust_state: DeviceTrustState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl DeviceRecord {
    pub fn to_summary(&self) -> DeviceSummary {
        DeviceSummary {
            id: self.id,
            signal_device_id: self.signal_device_id,
            installation_id: self.installation_id,
            platform: self.platform.clone(),
            label: self.label.clone(),
            trust_state: self.trust_state.as_str().to_string(),
            created_at: self.created_at,
            last_seen_at: self.last_seen_at,
            approved_at: self.approved_at,
            revoked_at: self.revoked_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncEvent {
    pub id: Uuid,
    pub event_type: String,
    pub source_device_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Wrap a [`SyncEvent`] into the JSON envelope sent over WebSocket.
pub fn sync_event_payload(event: &SyncEvent) -> serde_json::Value {
    serde_json::json!({
        "type": "device_sync_event",
        "id": event.id,
        "event_type": event.event_type.clone(),
        "source_device_id": event.source_device_id,
        "payload": event.payload.clone(),
        "created_at": event.created_at,
    })
}

/// Send a sync event directly to a device over WebSocket (no DB persistence).
///
/// Used for ephemeral notifications (e.g. `device_revoked`) where delivery
/// guarantees are not required — the event is fire-and-forget.
pub fn send_live_sync_event(
    target_device_id: Uuid,
    source_device_id: Option<Uuid>,
    event_type: &str,
    payload: serde_json::Value,
    state: &AppState,
) {
    let event = SyncEvent {
        id: Uuid::new_v4(),
        event_type: event_type.to_string(),
        source_device_id,
        payload,
        created_at: chrono::Utc::now(),
    };

    state.hub.send_to_device(
        &target_device_id,
        crate::hub::WsOutbound::Message {
            payload: sync_event_payload(&event),
        },
    );
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateTrustRequestBody {
    target_device_id: Option<Uuid>,
    sync_public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateSyncEventBody {
    target_device_id: Uuid,
    event_type: String,
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AckSyncEventsBody {
    event_ids: Vec<Uuid>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_devices))
        .route("/trust-requests", post(create_trust_request))
        .route("/sync-events", get(get_sync_events).post(create_sync_event))
        .route("/sync-events/ack", post(ack_sync_events))
        .route("/:id/approve", post(approve_device))
        .route("/:id", delete(revoke_device))
}

/// Register a new device or reactivate an existing one for the given user.
///
/// If `bootstrap.installation_id` matches an existing non-revoked device, that
/// device is reused (platform/label updated, `last_seen_at` bumped). Otherwise
/// a new device row is inserted.
///
/// The first device for a user is automatically trusted. Subsequent devices
/// start in `pending_trust` and a trust request is enqueued for approval by
/// an existing trusted device.
///
/// # Errors
///
/// * `AppError::BadRequest` — invalid platform/label in bootstrap payload.
/// * `AppError::Internal` — database error.
///
/// # Security invariants
///
/// * Only the first device is auto-trusted; all others require explicit approval
///   from an already-trusted device to prevent unauthorized device enrollment.
pub async fn register_or_reuse_device(
    user_id: Uuid,
    bootstrap: &DeviceBootstrap,
    state: &AppState,
) -> AppResult<DeviceRecord> {
    validate_bootstrap(bootstrap)?;

    if let Some(installation_id) = bootstrap.installation_id {
        // Try to find and update an existing device in a single query (saves 2 round-trips)
        let row = sqlx::query(
            r#"
            UPDATE devices
            SET platform = $3,
                label = $4,
                last_seen_at = NOW()
            WHERE user_id = $1 AND installation_id = $2 AND revoked_at IS NULL
            RETURNING id, user_id, signal_device_id, installation_id, platform, label,
                      trust_state, created_at, last_seen_at, approved_at, revoked_at
            "#,
        )
        .bind(user_id)
        .bind(installation_id)
        .bind(&bootstrap.platform)
        .bind(&bootstrap.label)
        .fetch_optional(state.db.pool())
        .await?;

        if let Some(row) = row {
            return device_from_row(&row);
        }
    }

    let trusted_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM devices
        WHERE user_id = $1 AND revoked_at IS NULL AND trust_state = 'trusted'
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await?;

    let signal_device_id = next_signal_device_id(user_id, state).await?;
    let trust_state = if trusted_count == 0 {
        DeviceTrustState::Trusted
    } else {
        DeviceTrustState::PendingTrust
    };

    let approved_at = if trust_state == DeviceTrustState::Trusted {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let row = sqlx::query(
        r#"
        INSERT INTO devices
            (user_id, signal_device_id, installation_id, platform, label, trust_state, last_seen_at, approved_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), $7)
        RETURNING id, user_id, signal_device_id, installation_id, platform, label,
                  trust_state, created_at, last_seen_at, approved_at, revoked_at
        "#,
    )
    .bind(user_id)
    .bind(signal_device_id)
    .bind(bootstrap.installation_id)
    .bind(&bootstrap.platform)
    .bind(&bootstrap.label)
    .bind(trust_state.as_str())
    .bind(approved_at)
    .fetch_one(state.db.pool())
    .await?;

    let device = device_from_row(&row)?;

    if device.trust_state == DeviceTrustState::PendingTrust {
        enqueue_trust_request(device.id, user_id, None, state).await?;
    }

    Ok(device)
}

/// Fetch a single device belonging to `user_id`, or 401 if not found.
///
/// # Security invariants
///
/// * Ownership check: returns `Unauthorized` if the device does not belong to
///   the given user, preventing cross-user device access (IDOR).
pub async fn get_device_for_user(
    user_id: Uuid,
    device_id: Uuid,
    state: &AppState,
) -> AppResult<DeviceRecord> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, signal_device_id, installation_id, platform, label,
               trust_state, created_at, last_seen_at, approved_at, revoked_at
        FROM devices
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(device_id)
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::Unauthorized)?;

    device_from_row(&row)
}

/// Bump `last_seen_at` for the given device to `NOW()`.
pub async fn touch_device(device_id: Uuid, state: &AppState) -> AppResult<()> {
    sqlx::query("UPDATE devices SET last_seen_at = NOW() WHERE id = $1")
        .bind(device_id)
        .execute(state.db.pool())
        .await?;
    Ok(())
}

/// Return all non-revoked, trusted devices for a user (ordered by creation date).
pub async fn trusted_devices_for_user(
    user_id: Uuid,
    state: &AppState,
) -> AppResult<Vec<DeviceRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, signal_device_id, installation_id, platform, label,
               trust_state, created_at, last_seen_at, approved_at, revoked_at
        FROM devices
        WHERE user_id = $1 AND revoked_at IS NULL AND trust_state = 'trusted'
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await?;

    rows.iter().map(device_from_row).collect()
}

/// Persist a device sync event to the database and deliver it live if the
/// target device is currently connected via WebSocket.
///
/// Unlike [`send_live_sync_event`], this provides at-least-once delivery:
/// events are stored in `device_sync_events` and re-delivered on reconnect
/// via [`take_sync_events`].
pub async fn enqueue_sync_event(
    user_id: Uuid,
    target_device_id: Uuid,
    source_device_id: Option<Uuid>,
    event_type: &str,
    payload: serde_json::Value,
    state: &AppState,
) -> AppResult<()> {
    let event_payload = payload.clone();
    let row = sqlx::query(
        r#"
        INSERT INTO device_sync_events
            (user_id, target_device_id, source_device_id, event_type, payload)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, created_at
        "#,
    )
    .bind(user_id)
    .bind(target_device_id)
    .bind(source_device_id)
    .bind(event_type)
    .bind(payload)
    .fetch_one(state.db.pool())
    .await?;

    let event = SyncEvent {
        id: row.try_get("id")?,
        event_type: event_type.to_string(),
        source_device_id,
        payload: event_payload,
        created_at: row.try_get("created_at")?,
    };

    if state.hub.is_device_online(&target_device_id) {
        state.hub.send_to_device(
            &target_device_id,
            crate::hub::WsOutbound::Message {
                payload: sync_event_payload(&event),
            },
        );
    }

    Ok(())
}

/// Fetch all undelivered sync events for `device_id`.
///
/// Delivery is acknowledged separately through [`ack_sync_events`] after the
/// client has actually processed the payload. Returning events from this
/// endpoint must not mutate delivery state, otherwise a transport failure
/// between response serialization and client receipt would permanently drop
/// sync events.
pub async fn take_sync_events(device_id: Uuid, state: &AppState) -> AppResult<Vec<SyncEvent>> {
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
    .await?;

    let events: Vec<SyncEvent> = rows
        .iter()
        .map(|row| {
            Ok(SyncEvent {
                id: row.try_get("id")?,
                event_type: row.try_get("event_type")?,
                source_device_id: row.try_get("source_device_id").ok().flatten(),
                payload: row.try_get("payload")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::from)?;

    Ok(events)
}

/// GET /api/v2/devices — list all devices (including revoked) for the authenticated user.
async fn list_devices(
    auth: AuthDevice,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DeviceSummary>>> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, signal_device_id, installation_id, platform, label,
               trust_state, created_at, last_seen_at, approved_at, revoked_at
        FROM devices
        WHERE user_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let devices = rows
        .iter()
        .map(device_from_row)
        .collect::<AppResult<Vec<_>>>()?
        .into_iter()
        .map(|device| device.to_summary())
        .collect();

    Ok(Json(devices))
}

/// POST /api/v2/devices/trust-requests — request trust approval for a device.
///
/// If the target device is already trusted, returns `{ "status": "already_trusted" }`.
/// Otherwise enqueues a trust request for approval by an existing trusted device.
async fn create_trust_request(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(body): Json<CreateTrustRequestBody>,
) -> AppResult<Json<serde_json::Value>> {
    let target = body.target_device_id.unwrap_or(auth.device_id);
    let target_device = get_device_for_user(auth.user_id, target, &state).await?;
    let sync_public_key = validate_sync_public_key(body.sync_public_key.as_deref())?;

    if target_device.trust_state == DeviceTrustState::Trusted {
        return Ok(Json(serde_json::json!({ "status": "already_trusted" })));
    }

    enqueue_trust_request(target, auth.user_id, sync_public_key.as_deref(), &state).await?;
    Ok(Json(
        serde_json::json!({ "status": "queued", "target_device_id": target }),
    ))
}

/// POST /api/v2/devices/:id/approve — approve a pending device from a trusted device.
///
/// # Security invariants
///
/// * Caller must be on a trusted device (`require_trusted`).
/// * Target device must belong to the same user (`get_device_for_user` ownership check).
/// * Revoked devices cannot be re-approved.
async fn approve_device(
    auth: AuthDevice,
    State(state): State<AppState>,
    Path(target_device_id): Path<Uuid>,
) -> AppResult<Json<DeviceSummary>> {
    auth.require_trusted()?;

    let target = get_device_for_user(auth.user_id, target_device_id, &state).await?;
    if target.revoked_at.is_some() {
        return Err(AppError::Conflict("Device is already revoked".into()));
    }

    sqlx::query(
        r#"
        UPDATE devices
        SET trust_state = 'trusted',
            approved_at = NOW(),
            approved_by = $1,
            last_seen_at = COALESCE(last_seen_at, NOW())
        WHERE id = $2 AND user_id = $3
        "#,
    )
    .bind(auth.device_id)
    .bind(target_device_id)
    .bind(auth.user_id)
    .execute(state.db.pool())
    .await?;

    sqlx::query(
        r#"
        UPDATE device_trust_requests
        SET approved_at = NOW(),
            approved_by = $1
        WHERE target_device_id = $2
        "#,
    )
    .bind(auth.device_id)
    .bind(target_device_id)
    .execute(state.db.pool())
    .await?;

    // Invalidate cached trust state so the hub picks up the change immediately
    state
        .hub
        .invalidate_trust_cache(auth.user_id, target_device_id);

    enqueue_sync_event(
        auth.user_id,
        target_device_id,
        Some(auth.device_id),
        "device_trust_approved",
        serde_json::json!({ "approved_by": auth.device_id }),
        &state,
    )
    .await?;

    enqueue_sync_event(
        auth.user_id,
        auth.device_id,
        Some(target_device_id),
        "device_trust_approved",
        serde_json::json!({ "target_device_id": target_device_id }),
        &state,
    )
    .await?;

    let updated = get_device_for_user(auth.user_id, target_device_id, &state).await?;
    Ok(Json(updated.to_summary()))
}

/// DELETE /api/v2/devices/:id — revoke a device, destroying its sessions and signal keys.
///
/// A device can revoke itself (self-logout) or a trusted device can revoke
/// another device belonging to the same user.
///
/// # Security invariants
///
/// * Cross-device revocation requires a trusted device (`require_trusted`).
/// * Cannot revoke the user's sole trusted device unless a password is set,
///   preventing permanent account lockout for OAuth-only users.
/// * Revocation deletes all signal keys (identity, signed prekeys, OPKs) and
///   the key backup for the device, ensuring no stale cryptographic material
///   remains on the server after revocation.
///
/// # E2EE contract
///
/// * Signal keys are hard-deleted, not soft-deleted. A revoked device cannot
///   be used to decrypt future messages or participate in key exchanges.
async fn revoke_device(
    auth: AuthDevice,
    State(state): State<AppState>,
    Path(target_device_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    if auth.device_id != target_device_id {
        auth.require_trusted()?;
    }

    let target = get_device_for_user(auth.user_id, target_device_id, &state).await?;
    if target.revoked_at.is_some() {
        return Ok(Json(serde_json::json!({ "status": "already_revoked" })));
    }

    // Guard: prevent revoking the sole trusted device if the user has no password,
    // as they would be permanently locked out with no way to re-authenticate.
    if target.trust_state == DeviceTrustState::Trusted {
        let trusted = trusted_devices_for_user(auth.user_id, &state).await?;
        if trusted.len() <= 1 {
            let has_password = sqlx::query_scalar::<_, bool>(
                "SELECT password_hash IS NOT NULL FROM users WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(auth.user_id)
            .fetch_optional(state.db.pool())
            .await?
            .unwrap_or(false);

            if !has_password {
                return Err(AppError::Conflict(
                    "Cannot revoke your only trusted device without a password set".into(),
                ));
            }
        }
    }

    sqlx::query(
        r#"
        UPDATE devices
        SET trust_state = 'revoked',
            revoked_at = NOW()
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(target_device_id)
    .bind(auth.user_id)
    .execute(state.db.pool())
    .await?;

    // Revoke all active sessions for this device
    sqlx::query(
        "UPDATE sessions SET revoked_at = NOW() WHERE device_id = $1 AND revoked_at IS NULL",
    )
    .bind(target_device_id)
    .execute(state.db.pool())
    .await?;

    // Clean up signal keys for the revoked device (identity keys, signed prekeys, OPKs, backup)
    sqlx::query("DELETE FROM one_time_prekeys WHERE user_id = $1 AND device_id = $2")
        .bind(auth.user_id)
        .bind(target.signal_device_id)
        .execute(state.db.pool())
        .await?;
    sqlx::query("DELETE FROM signed_prekeys WHERE user_id = $1 AND device_id = $2")
        .bind(auth.user_id)
        .bind(target.signal_device_id)
        .execute(state.db.pool())
        .await?;
    sqlx::query("DELETE FROM identity_keys WHERE user_id = $1 AND device_id = $2")
        .bind(auth.user_id)
        .bind(target.signal_device_id)
        .execute(state.db.pool())
        .await?;
    sqlx::query("DELETE FROM key_backups WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(state.db.pool())
        .await?;

    // Invalidate cached trust state so the hub rejects this device immediately
    state
        .hub
        .invalidate_trust_cache(auth.user_id, target_device_id);

    send_live_sync_event(
        target_device_id,
        Some(auth.device_id),
        "device_revoked",
        serde_json::json!({ "device_id": target_device_id }),
        &state,
    );

    for device in trusted_devices_for_user(auth.user_id, &state).await? {
        if device.id == target_device_id {
            continue;
        }
        enqueue_sync_event(
            auth.user_id,
            device.id,
            Some(auth.device_id),
            "device_revoked",
            serde_json::json!({ "device_id": target_device_id }),
            &state,
        )
        .await?;
    }

    Ok(Json(
        serde_json::json!({ "status": "revoked", "device_id": target_device_id }),
    ))
}

async fn get_sync_events(
    auth: AuthDevice,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<SyncEvent>>> {
    Ok(Json(take_sync_events(auth.device_id, &state).await?))
}

async fn ack_sync_events(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(body): Json<AckSyncEventsBody>,
) -> AppResult<impl IntoResponse> {
    if body.event_ids.is_empty() {
        return Ok(StatusCode::NO_CONTENT);
    }

    sqlx::query(
        "UPDATE device_sync_events \
         SET delivered_at = NOW() \
         WHERE target_device_id = $1 \
           AND id = ANY($2) \
           AND delivered_at IS NULL",
    )
    .bind(auth.device_id)
    .bind(&body.event_ids)
    .execute(state.db.pool())
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn create_sync_event(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(body): Json<CreateSyncEventBody>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_trusted()?;

    if !matches!(
        body.event_type.as_str(),
        "device_sync_chunk" | "device_sync_complete" | "device_revoked"
    ) {
        return Err(AppError::BadRequest("Invalid sync event type".into()));
    }

    let target = get_device_for_user(auth.user_id, body.target_device_id, &state).await?;
    if target.revoked_at.is_some() {
        return Err(AppError::Conflict("Target device is revoked".into()));
    }

    enqueue_sync_event(
        auth.user_id,
        body.target_device_id,
        Some(auth.device_id),
        &body.event_type,
        body.payload,
        &state,
    )
    .await?;

    Ok(Json(serde_json::json!({ "status": "queued" })))
}

async fn enqueue_trust_request(
    target_device_id: Uuid,
    user_id: Uuid,
    sync_public_key: Option<&str>,
    state: &AppState,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO device_trust_requests (user_id, target_device_id)
        VALUES ($1, $2)
        ON CONFLICT (target_device_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(target_device_id)
    .execute(state.db.pool())
    .await?;

    for device in trusted_devices_for_user(user_id, state).await? {
        enqueue_sync_event(
            user_id,
            device.id,
            Some(target_device_id),
            "device_trust_request",
            serde_json::json!({
                "target_device_id": target_device_id,
                "sync_public_key": sync_public_key,
            }),
            state,
        )
        .await?;
    }

    Ok(())
}

async fn next_signal_device_id(user_id: Uuid, state: &AppState) -> AppResult<i32> {
    let row = sqlx::query(
        r#"
        SELECT
            COALESCE((SELECT COUNT(*) FROM devices WHERE user_id = $1), 0) AS device_count,
            COALESCE((SELECT MAX(signal_device_id) FROM devices WHERE user_id = $1), 0) AS max_signal_device_id,
            COALESCE((SELECT MAX(device_id) FROM identity_keys WHERE user_id = $1), 0) AS max_identity_device_id
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await?;

    let device_count: i64 = row.try_get("device_count")?;
    let max_signal_device_id: i32 = row.try_get("max_signal_device_id")?;
    let max_identity_device_id: i32 = row.try_get("max_identity_device_id")?;

    if device_count == 0 && max_identity_device_id > 0 {
        return Ok(max_identity_device_id);
    }

    let max_device_id = max_signal_device_id.max(max_identity_device_id);
    Ok(if max_device_id == 0 {
        1
    } else {
        max_device_id + 1
    })
}

fn validate_bootstrap(bootstrap: &DeviceBootstrap) -> AppResult<()> {
    let platform = bootstrap.platform.trim();
    let label = bootstrap.label.trim();

    if !matches!(platform, "web" | "tauri" | "capacitor") {
        return Err(AppError::BadRequest("Invalid device platform".into()));
    }
    if label.is_empty() || label.len() > 64 {
        return Err(AppError::BadRequest(
            "Device label must be 1-64 characters".into(),
        ));
    }

    Ok(())
}

fn validate_sync_public_key(sync_public_key: Option<&str>) -> AppResult<Option<String>> {
    let Some(sync_public_key) = sync_public_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let bytes = BASE64
        .decode(sync_public_key)
        .map_err(|_| AppError::BadRequest("Invalid sync_public_key encoding".into()))?;
    if bytes.len() != 32 {
        return Err(AppError::BadRequest(
            "sync_public_key must be 32 bytes".into(),
        ));
    }

    Ok(Some(sync_public_key.to_string()))
}

fn device_from_row(row: &sqlx::postgres::PgRow) -> AppResult<DeviceRecord> {
    let trust_state_raw: String = row.try_get("trust_state")?;
    Ok(DeviceRecord {
        id: row.try_get("id")?,
        signal_device_id: row.try_get("signal_device_id")?,
        installation_id: row.try_get("installation_id").ok().flatten(),
        platform: row.try_get("platform")?,
        label: row.try_get("label")?,
        trust_state: DeviceTrustState::try_from(trust_state_raw.as_str())?,
        created_at: row.try_get("created_at")?,
        last_seen_at: row.try_get("last_seen_at").ok().flatten(),
        approved_at: row.try_get("approved_at").ok().flatten(),
        revoked_at: row.try_get("revoked_at").ok().flatten(),
    })
}

#[cfg(test)]
mod tests {
    use super::validate_sync_public_key;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    #[test]
    fn validate_sync_public_key_accepts_32_byte_key() {
        let key = BASE64.encode([7u8; 32]);
        assert_eq!(validate_sync_public_key(Some(&key)).unwrap(), Some(key));
    }

    #[test]
    fn validate_sync_public_key_rejects_wrong_length() {
        let key = BASE64.encode([7u8; 31]);
        assert!(validate_sync_public_key(Some(&key)).is_err());
    }
}
