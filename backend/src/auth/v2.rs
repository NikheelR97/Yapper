use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};
use serde::Serialize;
use sqlx::Row;
use std::net::SocketAddr;
use uuid::Uuid;
use validator::Validate;

use super::{
    handlers::{
        append_set_cookie_headers, auth_cookies_for_path, clear_auth_cookies_for_path, extract_ip,
        extract_refresh_cookie, send_verification_email, sha256_hex, store_session_with_claims,
        LoginRequest, RegisterRequest, UserDto, USERNAME_RE,
    },
    service::{
        generate_access_token, generate_email_token, generate_refresh_token, hash_password_async,
        validate_refresh_token, verify_password_async,
    },
    AuthUser,
};
use crate::{
    devices::{self, DeviceBootstrap, DeviceSummary},
    error::{AppError, AppResult},
    AppState,
};

const REFRESH_COOKIE_PATH_V2: &str = "/api/v2/auth/refresh";

#[derive(Debug, Serialize)]
struct AuthResponseV2 {
    access_token: String,
    csrf_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    user: UserDto,
    device: DeviceSummary,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/oauth/exchange", post(oauth_exchange))
        .route("/attach-device", post(attach_device))
        .route("/refresh", post(refresh))
        .route("/logout", delete(logout))
}

#[derive(Debug, serde::Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct RegisterRequestV2 {
    #[serde(flatten)]
    auth: RegisterRequest,
    device: DeviceBootstrap,
}

#[derive(Debug, serde::Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct LoginRequestV2 {
    #[serde(flatten)]
    auth: LoginRequest,
    device: DeviceBootstrap,
}

#[derive(Debug, serde::Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct OAuthExchangeRequest {
    code: String,
    device: DeviceBootstrap,
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequestV2>,
) -> AppResult<impl IntoResponse> {
    req.auth
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if !USERNAME_RE.is_match(&req.auth.username) {
        return Err(AppError::BadRequest(
            "Username may only contain letters, numbers and underscores".to_string(),
        ));
    }

    // COPPA: block direct registration for users under 13.
    if let Some(ref dob_str) = req.auth.date_of_birth {
        if let Ok(dob) = dob_str.parse::<chrono::NaiveDate>() {
            let today = chrono::Utc::now().date_naive();
            let age = today.years_since(dob).unwrap_or(0);
            if age < 13 {
                return Err(AppError::BadRequest(
                    "Users under 13 require a parent-managed account".to_string(),
                ));
            }
        }
    }

    let pool = state.db.pool();
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 OR email = $2)",
    )
    .bind(&req.auth.username)
    .bind(req.auth.email.to_lowercase())
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if exists {
        return Err(AppError::Conflict(
            "Username or email already taken".to_string(),
        ));
    }

    let password_hash = hash_password_async(req.auth.password.clone()).await?;
    let email_token = generate_email_token();
    let display_name = req
        .auth
        .display_name
        .clone()
        .unwrap_or_else(|| req.auth.username.clone());

    let user = sqlx::query_as::<_, UserDto>(
        r#"
        INSERT INTO users
            (username, email, display_name, password_hash, email_verify_token,
             email_verify_expires_at, gdpr_consent_at)
        VALUES ($1, $2, $3, $4, $5, NOW() + INTERVAL '24 hours', NOW())
        RETURNING id, username, display_name, avatar_url, account_type, is_premium
        "#,
    )
    .bind(&req.auth.username)
    .bind(req.auth.email.to_lowercase())
    .bind(display_name)
    .bind(password_hash)
    .bind(email_token.clone())
    .fetch_one(pool)
    .await?;

    let email = req.auth.email.clone();
    let resend_key = std::env::var("RESEND_API_KEY").unwrap_or_default();
    tokio::spawn(async move {
        let _ = send_verification_email(&email, &email_token, &resend_key).await;
    });

    let device = devices::register_or_reuse_device(user.id, &req.device, &state).await?;
    let response = issue_device_session(&state, user, device, Uuid::new_v4()).await?;
    Ok(response)
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequestV2>,
) -> AppResult<impl IntoResponse> {
    req.auth
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let ip = extract_ip(&headers, Some(peer_addr.ip()), &state);
    if state.login_limiter.is_locked(ip) {
        return Err(AppError::RateLimited);
    }

    let row = sqlx::query(
        r#"
        SELECT id, username, display_name, avatar_url, account_type, is_premium,
               password_hash, email_verified
        FROM users
        WHERE email = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(req.auth.email.to_lowercase())
    .fetch_optional(state.db.pool())
    .await?;

    let Some(row) = row else {
        state.login_limiter.record_failure(ip);
        return Err(AppError::Unauthorized);
    };

    let user_id: Uuid = row.try_get("id")?;
    let password_hash: Option<String> = row.try_get("password_hash").ok().flatten();
    let Some(hash) = password_hash else {
        state.login_limiter.record_failure(ip);
        return Err(AppError::BadRequest(
            "This account uses social login. Please sign in with Discord, Google, or Apple."
                .to_string(),
        ));
    };

    if !verify_password_async(req.auth.password.clone(), hash.clone()).await? {
        let locked = state.login_limiter.record_failure(ip);
        if locked {
            return Err(AppError::RateLimited);
        }
        return Err(AppError::Unauthorized);
    }

    state.login_limiter.record_success(ip);

    let user = UserDto {
        id: user_id,
        username: row.try_get("username")?,
        display_name: row.try_get("display_name")?,
        avatar_url: row.try_get("avatar_url").ok().flatten(),
        account_type: row.try_get("account_type")?,
        is_premium: row.try_get("is_premium").unwrap_or(false),
    };

    let device = devices::register_or_reuse_device(user.id, &req.device, &state).await?;
    let response = issue_device_session(&state, user, device, Uuid::new_v4()).await?;
    Ok(response)
}

async fn attach_device(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(device_bootstrap): Json<DeviceBootstrap>,
) -> AppResult<impl IntoResponse> {
    let user = sqlx::query_as::<_, UserDto>(
        "SELECT id, username, display_name, avatar_url, account_type, is_premium
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(AppError::Unauthorized)?;

    let device = devices::register_or_reuse_device(user.id, &device_bootstrap, &state).await?;
    let response = issue_device_session(&state, user, device, Uuid::new_v4()).await?;
    Ok(response)
}

async fn oauth_exchange(
    State(state): State<AppState>,
    Json(req): Json<OAuthExchangeRequest>,
) -> AppResult<impl IntoResponse> {
    let consumed = super::oauth::consume_oauth_login_code(&state, &req.code).await?;
    let user = sqlx::query_as::<_, UserDto>(
        "SELECT id, username, display_name, avatar_url, account_type, is_premium
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(consumed.user_id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(AppError::Unauthorized)?;

    let device = devices::register_or_reuse_device(user.id, &req.device, &state).await?;
    let response = issue_device_session(&state, user, device, Uuid::new_v4()).await?;
    Ok(response)
}

async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    let refresh_token = extract_refresh_cookie(&headers).ok_or(AppError::Unauthorized)?;
    let claims = validate_refresh_token(&refresh_token, &state.jwt_keys)?.claims;
    let device_id = claims.device_id.ok_or(AppError::Unauthorized)?;
    let token_hash = sha256_hex(&refresh_token);

    let revoked = sqlx::query(
        r#"
        UPDATE sessions
        SET revoked_at = NOW()
        WHERE token_hash = $1 AND user_id = $2 AND device_id = $3 AND revoked_at IS NULL
        RETURNING family_id
        "#,
    )
    .bind(&token_hash)
    .bind(claims.sub)
    .bind(device_id)
    .fetch_optional(state.db.pool())
    .await?;

    let family_id = match revoked {
        Some(row) => row.try_get("family_id")?,
        None => {
            let existing = sqlx::query(
                r#"
                SELECT family_id, revoked_at
                FROM sessions
                WHERE token_hash = $1 AND user_id = $2 AND device_id = $3
                "#,
            )
            .bind(&token_hash)
            .bind(claims.sub)
            .bind(device_id)
            .fetch_optional(state.db.pool())
            .await?;

            if let Some(row) = existing {
                let family_id: Uuid = row.try_get("family_id")?;
                let revoked_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("revoked_at").unwrap_or(None);
                
                // If revoked very recently (e.g. <30s), treat as benign concurrent client retry,
                // fail this request but don't nuke the session family.
                let is_recent_retry = revoked_at
                    .map(|r| chrono::Utc::now().signed_duration_since(r).num_seconds() < 30)
                    .unwrap_or(false);

                if !is_recent_retry {
                    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE family_id = $1 AND revoked_at IS NULL")
                        .bind(family_id)
                        .execute(state.db.pool())
                        .await?;
                    tracing::warn!(
                        "Refresh token reuse detected for user {} device {:?}. Family {} invalidated.",
                        claims.sub,
                        device_id,
                        family_id
                    );
                }
            }
            return Err(AppError::Unauthorized);
        }
    };

    let user = sqlx::query_as::<_, UserDto>(
        "SELECT id, username, display_name, avatar_url, account_type, is_premium
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(claims.sub)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(AppError::Unauthorized)?;

    let device = devices::get_device_for_user(user.id, device_id, &state).await?;
    if device.revoked_at.is_some() {
        return Err(AppError::Unauthorized);
    }

    let access_token = generate_access_token(
        user.id,
        &user.account_type,
        Some(device.id),
        &state.jwt_keys,
    )?;
    let refresh_token =
        generate_refresh_token(user.id, family_id, Some(device.id), &state.jwt_keys)?;
    let exp = (chrono::Utc::now() + chrono::Duration::seconds(super::service::REFRESH_TTL_SECS))
        .timestamp();
    store_session_with_claims(
        state.db.pool(),
        user.id,
        &refresh_token,
        Some(device.id),
        family_id,
        exp,
    )
    .await?;
    devices::touch_device(device.id, &state).await?;

    let (cookies, csrf_token) = auth_cookies_for_path(&refresh_token, REFRESH_COOKIE_PATH_V2);

    // Only expose refresh_token in JSON for native clients (Tauri/Capacitor) where
    // cross-origin cookies are unreliable. Web browsers use the HttpOnly cookie instead,
    // keeping the token out of JavaScript reach (XSS mitigation).
    let json_refresh_token = if device.platform == "tauri" || device.platform == "capacitor" {
        Some(refresh_token)
    } else {
        None
    };

    Ok((
        append_set_cookie_headers(&cookies),
        Json(AuthResponseV2 {
            access_token,
            csrf_token,
            refresh_token: json_refresh_token,
            user,
            device: device.to_summary(),
        }),
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<impl IntoResponse> {
    if let Some(refresh_token) = extract_refresh_cookie(&headers) {
        if let Ok(claims) = validate_refresh_token(&refresh_token, &state.jwt_keys) {
            let token_hash = sha256_hex(&refresh_token);
            sqlx::query(
                r#"
                UPDATE sessions SET revoked_at = NOW()
                WHERE family_id = (
                    SELECT family_id FROM sessions
                    WHERE token_hash = $1 AND user_id = $2 AND device_id = $3
                    LIMIT 1
                )
                "#,
            )
            .bind(token_hash)
            .bind(claims.claims.sub)
            .bind(claims.claims.device_id)
            .execute(state.db.pool())
            .await?;
        }
    }

    let cookies = clear_auth_cookies_for_path(REFRESH_COOKIE_PATH_V2);
    Ok((
        append_set_cookie_headers(&cookies),
        axum::http::StatusCode::NO_CONTENT,
    ))
}

pub(super) async fn issue_device_session(
    state: &AppState,
    user: UserDto,
    device: devices::DeviceRecord,
    family_id: Uuid,
) -> AppResult<impl IntoResponse> {
    let access_token = generate_access_token(
        user.id,
        &user.account_type,
        Some(device.id),
        &state.jwt_keys,
    )?;
    let refresh_token =
        generate_refresh_token(user.id, family_id, Some(device.id), &state.jwt_keys)?;

    let exp = (chrono::Utc::now() + chrono::Duration::seconds(super::service::REFRESH_TTL_SECS))
        .timestamp();
    store_session_with_claims(
        state.db.pool(),
        user.id,
        &refresh_token,
        Some(device.id),
        family_id,
        exp,
    )
    .await?;
    // Note: touch_device is skipped here because register_or_reuse_device
    // already set last_seen_at = NOW() during device creation/update.

    let (cookies, csrf_token) = auth_cookies_for_path(&refresh_token, REFRESH_COOKIE_PATH_V2);

    // Only expose refresh_token in JSON for native clients (Tauri/Capacitor) where
    // cross-origin cookies are unreliable. Web browsers use the HttpOnly cookie instead,
    // keeping the token out of JavaScript reach (XSS mitigation).
    let json_refresh_token = if device.platform == "tauri" || device.platform == "capacitor" {
        Some(refresh_token)
    } else {
        None
    };

    Ok((
        append_set_cookie_headers(&cookies),
        Json(AuthResponseV2 {
            access_token,
            csrf_token,
            refresh_token: json_refresh_token,
            user,
            device: device.to_summary(),
        }),
    ))
}
