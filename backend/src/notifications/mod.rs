/**
 * Push notifications — register FCM/APNS tokens and send push via FCM HTTP v1.
 *
 * Routes (mounted under /api/v1/notifications):
 *   PUT    /push-token  — register or update push token for the current device
 *   DELETE /push-token  — remove push token for the current device
 *
 * FCM integration:
 *   - Requires FCM_SERVICE_ACCOUNT_JSON env var (path or inline JSON).
 *   - Uses FCM HTTP v1 API with OAuth2 service account token.
 *   - Data-only messages (no notification block): clients handle display.
 *   - E2EE: only metadata is sent (sender_id, conversation_id, type).
 */
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, put},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use sqlx::Row;
use std::sync::RwLock;
use uuid::Uuid;

use crate::{
    auth::AuthDevice,
    error::{AppError, AppResult},
    AppState,
};

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/push-token", put(register_push_token))
        .route("/push-token", delete(remove_push_token))
}

// ─── Input types ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterPushTokenReq {
    /// FCM registration token or APNS device token
    token: String,
    /// "fcm" | "apns" | "web"
    platform: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

async fn register_push_token(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(req): Json<RegisterPushTokenReq>,
) -> AppResult<impl IntoResponse> {
    auth.require_trusted()?;

    if !matches!(req.platform.as_str(), "fcm" | "apns" | "web") {
        return Err(AppError::BadRequest(
            "platform must be one of: fcm, apns, web".into(),
        ));
    }
    if req.token.is_empty() || req.token.len() > 4096 {
        return Err(AppError::BadRequest(
            "token must be 1–4096 characters".into(),
        ));
    }

    // Enforce per-user push token cap: if at limit, remove the oldest token
    // before registering the new one. Prevents unbounded token accumulation.
    let token_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM devices WHERE user_id = $1 AND push_token IS NOT NULL AND revoked_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_one(state.db.pool())
    .await?;

    if token_count >= crate::constants::MAX_PUSH_TOKENS_PER_USER {
        sqlx::query(
            r#"
            UPDATE devices SET push_token = NULL, push_platform = NULL
            WHERE id = (
                SELECT id FROM devices
                WHERE user_id = $1 AND push_token IS NOT NULL AND revoked_at IS NULL AND id != $2
                ORDER BY last_seen_at ASC NULLS FIRST
                LIMIT 1
            )
            "#,
        )
        .bind(auth.user_id)
        .bind(auth.device_id)
        .execute(state.db.pool())
        .await?;
    }

    sqlx::query("UPDATE devices SET push_token = $1, push_platform = $2 WHERE id = $3")
        .bind(&req.token)
        .bind(&req.platform)
        .bind(auth.device_id)
        .execute(state.db.pool())
        .await?;

    tracing::info!(
        device_id = %auth.device_id,
        platform = %req.platform,
        "Push token registered"
    );

    Ok(StatusCode::NO_CONTENT)
}

async fn remove_push_token(
    auth: AuthDevice,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    auth.require_trusted()?;

    sqlx::query("UPDATE devices SET push_token = NULL, push_platform = NULL WHERE id = $1")
        .bind(auth.device_id)
        .execute(state.db.pool())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ─── FCM HTTP v1 push delivery ──────────────────────────────────────────────

/// Cached OAuth2 access token for FCM.
struct CachedToken {
    token: String,
    expires_at: std::time::Instant,
}

static FCM_TOKEN_CACHE: Lazy<RwLock<Option<CachedToken>>> = Lazy::new(|| RwLock::new(None));

/// FCM service account credentials (parsed from JSON).
#[derive(Deserialize)]
struct ServiceAccount {
    project_id: String,
    client_email: String,
    private_key: String,
}

fn load_service_account() -> Result<ServiceAccount, String> {
    let json = std::env::var("FCM_SERVICE_ACCOUNT_JSON")
        .map_err(|_| "FCM_SERVICE_ACCOUNT_JSON not configured".to_string())?;

    // If the value is a file path, read the file
    let json_content = if json.starts_with('{') {
        json
    } else {
        std::fs::read_to_string(&json)
            .map_err(|e| format!("Failed to read service account file: {e}"))?
    };

    serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse service account JSON: {e}"))
}

/// Obtain a short-lived OAuth2 access token using the service account JWT.
async fn get_fcm_access_token(http: &reqwest::Client) -> Result<String, String> {
    // Check cache first
    if let Ok(cache) = FCM_TOKEN_CACHE.read() {
        if let Some(ref cached) = *cache {
            if cached.expires_at > std::time::Instant::now() {
                return Ok(cached.token.clone());
            }
        }
    }

    let sa = load_service_account()?;
    let now = chrono::Utc::now().timestamp();

    let claims = serde_json::json!({
        "iss": sa.client_email,
        "scope": "https://www.googleapis.com/auth/firebase.messaging",
        "aud": "https://oauth2.googleapis.com/token",
        "iat": now,
        "exp": now + 3600,
    });

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .map_err(|e| format!("Invalid service account private key: {e}"))?;
    let jwt = jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| format!("Failed to sign JWT: {e}"))?;

    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()
        .await
        .map_err(|e| format!("OAuth2 token request failed: {e}"))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OAuth2 token error: {text}"));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("OAuth2 response parse failed: {e}"))?;

    let access_token = body["access_token"]
        .as_str()
        .ok_or("Missing access_token in OAuth2 response")?
        .to_string();

    let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

    // Cache with 5-minute safety margin
    if let Ok(mut cache) = FCM_TOKEN_CACHE.write() {
        *cache = Some(CachedToken {
            token: access_token.clone(),
            expires_at: std::time::Instant::now()
                + std::time::Duration::from_secs(expires_in.saturating_sub(300)),
        });
    }

    Ok(access_token)
}

/// Send a data-only push notification to a specific FCM token.
/// E2EE safe: only metadata (type, sender, conversation) is included.
pub async fn send_push(
    http: &reqwest::Client,
    fcm_token: &str,
    data: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let sa = load_service_account()?;
    let access_token = get_fcm_access_token(http).await?;

    let url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        sa.project_id
    );

    let message = serde_json::json!({
        "message": {
            "token": fcm_token,
            "data": data,
            "android": { "priority": "high" },
            "apns": {
                "headers": { "apns-priority": "10" },
                "payload": {
                    "aps": { "content-available": 1 }
                }
            },
            "webpush": {
                "headers": { "Urgency": "high" }
            }
        }
    });

    let resp = http
        .post(&url)
        .bearer_auth(&access_token)
        .json(&message)
        .send()
        .await
        .map_err(|e| format!("FCM send failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("FCM returned {status}: {text}"));
    }

    Ok(())
}

/// Send push notifications to all offline devices for a given user.
/// Best-effort: logs failures but doesn't propagate them.
pub async fn notify_user_offline_devices(
    user_id: Uuid,
    notification_type: &str,
    metadata: &std::collections::HashMap<String, String>,
    state: &AppState,
) {
    let rows = sqlx::query(
        "SELECT push_token, push_platform FROM devices \
         WHERE user_id = $1 AND push_token IS NOT NULL AND revoked_at IS NULL",
    )
    .bind(user_id)
    .fetch_all(state.db.pool())
    .await;

    let Ok(rows) = rows else { return };

    for row in &rows {
        let Ok(token) = row.try_get::<String, _>("push_token") else {
            continue;
        };

        let mut data = metadata.clone();
        data.insert("type".to_string(), notification_type.to_string());

        if let Err(e) = send_push(&state.http_client, &token, &data).await {
            tracing::warn!(
                user_id = %user_id,
                "Push notification failed: {e}"
            );
        }
    }
}
