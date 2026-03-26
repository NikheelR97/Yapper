use axum::{
    extract::{ConnectInfo, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse},
    Json,
};
use chrono::Utc;
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, RateLimiter};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use uuid::Uuid;
use validator::Validate;

use super::service::{
    generate_email_token, hash_password_async, verify_password_async, REFRESH_TTL_SECS,
};
use crate::{
    error::{AppError, AppResult},
    AppState,
};

// â”€â”€â”€ DTOs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {
    #[validate(length(min = 2, max = 32))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 1024))]
    pub password: String,

    pub display_name: Option<String>,

    /// Date of birth (YYYY-MM-DD). Required for COPPA compliance â€”
    /// users under 13 must be created through the parental child-account flow.
    pub date_of_birth: Option<String>,
}

// Username: alphanumeric + underscores, no spaces
pub(crate) static USERNAME_RE: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap());

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 1, max = 1024))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub csrf_token: String,
    pub user: UserDto,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserDto {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub account_type: String,
    pub is_premium: bool,
}

type KeyedLimiter<T> = RateLimiter<T, DefaultKeyedStateStore<T>, DefaultClock>;

static PASSWORD_RESET_IP_LIMITER: once_cell::sync::Lazy<KeyedLimiter<IpAddr>> =
    once_cell::sync::Lazy::new(|| {
        RateLimiter::keyed(
            governor::Quota::with_period(std::time::Duration::from_secs(15 * 60))
                .expect("valid password reset ip quota")
                .allow_burst(NonZeroU32::new(3).expect("non-zero burst")),
        )
    });
static PASSWORD_RESET_EMAIL_LIMITER: once_cell::sync::Lazy<KeyedLimiter<String>> =
    once_cell::sync::Lazy::new(|| {
        RateLimiter::keyed(
            governor::Quota::with_period(std::time::Duration::from_secs(15 * 60))
                .expect("valid password reset email quota")
                .allow_burst(NonZeroU32::new(3).expect("non-zero burst")),
        )
    });
static EMAIL_VERIFY_IP_LIMITER: once_cell::sync::Lazy<KeyedLimiter<IpAddr>> =
    once_cell::sync::Lazy::new(|| {
        RateLimiter::keyed(
            governor::Quota::with_period(std::time::Duration::from_secs(15 * 60))
                .expect("valid email verification quota")
                .allow_burst(NonZeroU32::new(10).expect("non-zero burst")),
        )
    });

/// Evict stale entries from the auth-related keyed rate limiters
/// (password reset IP/email and email verification IP).
///
/// Called periodically from the global GC task to prevent unbounded memory growth
/// in the in-memory `governor` keyed state stores.
pub fn gc_auth_rate_limiters() {
    PASSWORD_RESET_IP_LIMITER.retain_recent();
    PASSWORD_RESET_EMAIL_LIMITER.retain_recent();
    EMAIL_VERIFY_IP_LIMITER.retain_recent();
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyEmailQuery {
    pub token: String,
}



// â”€â”€â”€ Register â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// POST /api/v2/auth/verify-email â€” mark the account as email-verified using a one-time token.
pub async fn verify_email(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<VerifyEmailQuery>,
) -> AppResult<impl IntoResponse> {
    let ip = extract_ip(&headers, Some(peer_addr.ip()), &state);
    if EMAIL_VERIFY_IP_LIMITER.check_key(&ip).is_err() {
        return Err(AppError::RateLimited);
    }

    let rows_updated = sqlx::query!(
        r#"
        UPDATE users
        SET email_verified = TRUE,
            email_verify_token = NULL,
            email_verify_expires_at = NULL
        WHERE email_verify_token = $1
          AND email_verify_expires_at > NOW()
          AND email_verified = FALSE
        "#,
        q.token,
    )
    .execute(state.db.pool())
    .await?
    .rows_affected();

    if rows_updated == 0 {
        return Err(AppError::BadRequest(
            "Invalid or expired verification token".to_string(),
        ));
    }

    Ok(Json(
        serde_json::json!({ "message": "Email verified successfully" }),
    ))
}

// â”€â”€â”€ Password reset request â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PasswordResetRequest {
    #[validate(email)]
    pub email: String,
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<PasswordResetRequest>,
) -> AppResult<impl IntoResponse> {
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let ip = extract_ip(&headers, Some(peer_addr.ip()), &state);
    let normalized_email = req.email.trim().to_lowercase();
    if PASSWORD_RESET_IP_LIMITER.check_key(&ip).is_err()
        || PASSWORD_RESET_EMAIL_LIMITER
            .check_key(&normalized_email)
            .is_err()
    {
        return Err(AppError::RateLimited);
    }

    // Always return 200 â€” don't reveal whether email exists
    let pool = state.db.pool();
    let token = generate_email_token();

    let updated = sqlx::query!(
        r#"
        UPDATE users
        SET password_reset_token = $1, password_reset_expires_at = NOW() + INTERVAL '1 hour'
        WHERE email = $2 AND deleted_at IS NULL
        "#,
        token,
        normalized_email,
    )
    .execute(pool)
    .await?
    .rows_affected();

    if updated > 0 {
        let resend_key = std::env::var("RESEND_API_KEY").unwrap_or_default();
        let email = req.email.clone();
        tokio::spawn(async move {
            let _ = send_password_reset_email(&email, &token, &resend_key).await;
        });
    }

    Ok(Json(serde_json::json!({
        "message": "If that email is registered, a reset link has been sent"
    })))
}

// â”€â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Store a session row using pre-extracted claims, avoiding redundant JWT validation.
pub(crate) async fn store_session_with_claims(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    refresh_token: &str,
    device_id: Option<Uuid>,
    family_id: Uuid,
    exp: i64,
) -> AppResult<()> {
    let token_hash = sha256_hex(refresh_token);
    let expires_at = chrono::DateTime::from_timestamp(exp, 0).unwrap_or_else(Utc::now);

    sqlx::query(
        r#"
        INSERT INTO sessions (user_id, family_id, token_hash, expires_at, device_id)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id)
    .bind(family_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(device_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(crate) fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn auth_cookies_for_path(
    refresh_token: &str,
    refresh_cookie_path: &str,
) -> ([String; 2], String) {
    let csrf_token = Uuid::new_v4().to_string();
    let use_secure_cookie = crate::csrf::should_use_secure_cookie();
    let cookies = [
        refresh_cookie_header_for_path(
            refresh_token,
            REFRESH_TTL_SECS,
            use_secure_cookie,
            refresh_cookie_path,
        ),
        crate::csrf::csrf_cookie_header(&csrf_token),
    ];
    (cookies, csrf_token)
}

fn clear_auth_cookies() -> [String; 2] {
    clear_auth_cookies_for_path("/api/v2/auth/refresh")
}

pub(crate) fn clear_auth_cookies_for_path(refresh_cookie_path: &str) -> [String; 2] {
    let use_secure_cookie = crate::csrf::should_use_secure_cookie();
    [
        refresh_cookie_header_for_path("", 0, use_secure_cookie, refresh_cookie_path),
        crate::csrf::clear_csrf_cookie(),
    ]
}

pub(crate) fn extract_refresh_cookie(headers: &HeaderMap) -> Option<String> {
    // First try the cookie (web browsers)
    if let Some(token) = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            raw.split(';')
                .find(|s| s.trim().starts_with("refresh_token="))
                .map(|s| s.trim().trim_start_matches("refresh_token=").to_string())
        })
    {
        return Some(token);
    }
    // Fallback: X-Refresh-Token header (Tauri/Capacitor where cookies are blocked)
    headers
        .get("x-refresh-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub(crate) fn refresh_cookie_header_for_path(
    refresh_token: &str,
    max_age: i64,
    use_secure_cookie: bool,
    cookie_path: &str,
) -> String {
    let secure_flag = if use_secure_cookie { "; Secure" } else { "" };
    let same_site = if use_secure_cookie { "None" } else { "Strict" };
    format!(
        "refresh_token={refresh_token}; HttpOnly{secure_flag}; SameSite={same_site}; Path={cookie_path}; Max-Age={max_age}"
    )
}

pub(crate) fn append_set_cookie_headers(
    cookies: &[String; 2],
) -> AppendHeaders<[(header::HeaderName, String); 2]> {
    AppendHeaders([
        (header::SET_COOKIE, cookies[0].clone()),
        (header::SET_COOKIE, cookies[1].clone()),
    ])
}

pub(crate) fn extract_ip(headers: &HeaderMap, peer_ip: Option<IpAddr>, state: &AppState) -> IpAddr {
    let peer_ip = peer_ip.unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if std::env::var("FLY_APP_NAME").is_ok() {
        // Production: behind Fly.io load balancer â€” only trust Fly-Client-IP
        // (set by Fly.io proxy; clients cannot forge it)
        return headers
            .get("Fly-Client-IP")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(peer_ip);
    }

    // Local dev / other envs: trust proxy headers only from known proxy IPs
    if state.trusted_proxy_ips.contains(&peer_ip) {
        if let Some(ip) = headers
            .get("CF-Connecting-IP")
            .or_else(|| headers.get("X-Forwarded-For"))
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(str::trim)
            .and_then(|s| s.parse().ok())
        {
            return ip;
        }
    }

    peer_ip
}

pub(crate) async fn send_verification_email(
    to: &str,
    token: &str,
    api_key: &str,
) -> anyhow::Result<()> {
    if api_key.is_empty() {
        tracing::debug!("RESEND_API_KEY not set â€” skipping email to {to}");
        return Ok(());
    }

    static HTTP: once_cell::sync::Lazy<reqwest::Client> =
        once_cell::sync::Lazy::new(reqwest::Client::new);

    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    HTTP.post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "from": std::env::var("EMAIL_FROM").unwrap_or_else(|_| "Yapper <hello@yapperhq.com>".to_string()),
            "to": to,
            "subject": "Verify your Yapper email",
            "html": format!(
                "<p>Click the link to verify your email:</p>\
                 <p><a href=\"{frontend_url}/verify-email?token={token}\">Verify Email</a></p>\
                 <p>This link expires in 24 hours.</p>"
            )
        }))
        .send()
        .await?;

    Ok(())
}

async fn send_password_reset_email(to: &str, token: &str, api_key: &str) -> anyhow::Result<()> {
    if api_key.is_empty() {
        return Ok(());
    }

    static HTTP: once_cell::sync::Lazy<reqwest::Client> =
        once_cell::sync::Lazy::new(reqwest::Client::new);

    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    HTTP.post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "from": std::env::var("EMAIL_FROM").unwrap_or_else(|_| "Yapper <hello@yapperhq.com>".to_string()),
            "to": to,
            "subject": "Reset your Yapper password",
            "html": format!(
                "<p>Click the link to reset your password:</p>\
                 <p><a href=\"{frontend_url}/reset-password?token={token}\">Reset Password</a></p>\
                 <p>This link expires in 1 hour. If you didn't request this, ignore this email.</p>"
            )
        }))
        .send()
        .await?;

    Ok(())
}

// â”€â”€â”€ Password reset confirm â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PasswordResetConfirm {
    pub token: String,

    #[validate(length(min = 8, max = 1024))]
    pub new_password: String,
}

pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetConfirm>,
) -> AppResult<impl IntoResponse> {
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if req.token.is_empty() {
        return Err(AppError::BadRequest("Token is required".to_string()));
    }

    let new_hash = hash_password_async(req.new_password.clone()).await?;

    let updated = sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1,
            password_reset_token = NULL,
            password_reset_expires_at = NULL
        WHERE password_reset_token = $2
          AND password_reset_expires_at > NOW()
          AND deleted_at IS NULL
        RETURNING id
        "#,
        new_hash,
        req.token,
    )
    .fetch_optional(state.db.pool())
    .await?;

    let Some(updated) = updated else {
        return Err(AppError::BadRequest(
            "Invalid or expired reset token".to_string(),
        ));
    };

    sqlx::query!(
        "UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1",
        updated.id
    )
    .execute(state.db.pool())
    .await?;

    Ok(Json(
        serde_json::json!({ "message": "Password updated successfully" }),
    ))
}

// Change password (authenticated)
#[derive(Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    #[validate(length(min = 8, max = 1024))]
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<impl IntoResponse> {
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if req.current_password == req.new_password {
        return Err(AppError::BadRequest(
            "New password must differ from current password".to_string(),
        ));
    }

    let pool = state.db.pool();
    let row = sqlx::query!(
        "SELECT password_hash FROM users WHERE id = $1 AND deleted_at IS NULL",
        auth.user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let Some(current_hash) = row.password_hash else {
        return Err(AppError::BadRequest(
            "This account uses social login and has no password.".to_string(),
        ));
    };

    if !verify_password_async(req.current_password.clone(), current_hash.clone()).await? {
        return Err(AppError::Unauthorized);
    }

    let new_hash = hash_password_async(req.new_password.clone()).await?;
    sqlx::query!(
        "UPDATE users SET password_hash = $2 WHERE id = $1",
        auth.user_id,
        new_hash
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1",
        auth.user_id
    )
    .execute(pool)
    .await?;

    let cookies = clear_auth_cookies();
    Ok((append_set_cookie_headers(&cookies), StatusCode::NO_CONTENT))
}

#[cfg(test)]
mod tests {
    use super::refresh_cookie_header_for_path;

    #[test]
    fn refresh_cookie_header_omits_secure_flag_for_local_http() {
        let header = refresh_cookie_header_for_path("token", 60, false, "/api/v2/auth/refresh");

        assert!(header.contains("refresh_token=token"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("SameSite=Strict"));
        assert!(!header.contains("; Secure"));
    }

    #[test]
    fn refresh_cookie_header_includes_secure_flag_for_https() {
        let header = refresh_cookie_header_for_path("token", 60, true, "/api/v2/auth/refresh");

        assert!(header.contains("refresh_token=token"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("; Secure"));
        assert!(header.contains("SameSite=None"));
    }
}
