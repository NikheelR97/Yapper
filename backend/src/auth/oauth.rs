use axum::{
    extract::{Form, Query, State},
    http::{header, HeaderMap},
    response::{AppendHeaders, IntoResponse, Redirect},
};
use chrono::Utc;
use serde::{Deserialize, Deserializer};
use sqlx::Row;
use std::time::Instant;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

// â”€â”€â”€ Discord user info â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    discriminator: String,
    email: Option<String>,
    avatar: Option<String>,
    verified: Option<bool>,
}

// â”€â”€â”€ Google user info â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Deserialize)]
struct GoogleUser {
    id: String,
    email: String,
    name: String,
    picture: Option<String>,
    verified_email: Option<bool>,
}

// â”€â”€â”€ OAuth state store â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Short-lived state tokens keyed by the random state value.
pub type OAuthStateStore = dashmap::DashMap<String, Instant>;

const OAUTH_STATE_COOKIE_PATH: &str = "/auth/oauth";

fn api_base() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

fn frontend_base() -> String {
    std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string())
}

fn new_state_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    crate::hex_encode(&bytes)
}

fn new_oauth_login_code() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    crate::hex_encode(&bytes)
}

/// Remove state tokens older than 10 minutes to avoid unbounded growth.
fn gc_oauth_states(store: &OAuthStateStore) {
    let cutoff = Instant::now();
    store.retain(|_, created_at| cutoff.duration_since(*created_at).as_secs() < 600);
}

fn oauth_state_cookie_header(token: &str) -> String {
    let secure_flag = if crate::csrf::should_use_secure_cookie() {
        "; Secure"
    } else {
        ""
    };
    let same_site = if crate::csrf::should_use_secure_cookie() {
        "None"
    } else {
        "Lax"
    };
    format!(
        "oauth_state={token}; HttpOnly{secure_flag}; SameSite={same_site}; Path={OAUTH_STATE_COOKIE_PATH}; Max-Age=600"
    )
}

fn clear_oauth_state_cookie() -> String {
    let secure_flag = if crate::csrf::should_use_secure_cookie() {
        "; Secure"
    } else {
        ""
    };
    let same_site = if crate::csrf::should_use_secure_cookie() {
        "None"
    } else {
        "Lax"
    };
    format!(
        "oauth_state=; HttpOnly{secure_flag}; SameSite={same_site}; Path={OAUTH_STATE_COOKIE_PATH}; Max-Age=0"
    )
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie_header
        .split(';')
        .map(str::trim)
        .find(|part| part.starts_with(name))
        .map(|part| part.trim_start_matches(name).to_string())
}

fn validate_oauth_state(
    headers: &HeaderMap,
    expected_state: &str,
    state_store: &OAuthStateStore,
) -> bool {
    let cookie_state = cookie_value(headers, "oauth_state=");
    cookie_state.as_deref() == Some(expected_state) && state_store.remove(expected_state).is_some()
}

// â”€â”€â”€ Redirect + Callback query params â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Optional query params on the initial redirect endpoint (e.g. `?platform=desktop`).
#[derive(Debug, Deserialize, Default)]
pub struct OAuthRedirectQuery {
    platform: Option<String>,
}

impl OAuthRedirectQuery {
    fn is_desktop(&self) -> bool {
        self.platform.as_deref() == Some("desktop")
    }
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OAuthErrorParams {
    error: Option<String>,
    error_description: Option<String>,
}

// â”€â”€â”€ Discord OAuth â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub async fn discord_redirect(
    State(state): State<AppState>,
    Query(query): Query<OAuthRedirectQuery>,
) -> impl IntoResponse {
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_else(|_| "missing".to_string());
    let redirect_uri = format!("{}/auth/oauth/discord/callback", api_base());

    let csrf = new_state_token();
    // Embed `:desktop` suffix in the state token so the callback knows which frontend to redirect to.
    let state_token = if query.is_desktop() {
        format!("{csrf}:desktop")
    } else {
        csrf
    };
    gc_oauth_states(&state.oauth_states);
    state
        .oauth_states
        .insert(state_token.clone(), Instant::now());

    let auth_url = url::Url::parse_with_params(
        "https://discord.com/api/oauth2/authorize",
        &[
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "identify email"),
            ("state", state_token.as_str()),
        ],
    )
    .expect("Discord auth URL is valid");

    (
        AppendHeaders([(header::SET_COOKIE, oauth_state_cookie_header(&state_token))]),
        Redirect::to(auth_url.as_str()),
    )
        .into_response()
}

pub async fn discord_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<OAuthCallbackParams>,
) -> impl IntoResponse {
    // Validate CSRF state
    if !validate_oauth_state(&headers, &params.state, &state.oauth_states) {
        return oauth_error_redirect("invalid_state");
    }
    let is_desktop = params.state.ends_with(":desktop");

    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("DISCORD_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = format!("{}/auth/oauth/discord/callback", api_base());

    // Exchange code for access token
    let http = &state.http_client;
    let token_res = match http
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", params.code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Err(_) => return oauth_error_redirect("token_exchange_failed"),
        Ok(_) => return oauth_error_redirect("token_exchange_failed"),
    };

    #[derive(Deserialize)]
    struct DiscordTokenResponse {
        access_token: String,
    }

    let token: DiscordTokenResponse = match token_res.json().await {
        Ok(t) => t,
        Err(_) => return oauth_error_redirect("token_parse_failed"),
    };

    // Fetch Discord user profile
    let discord_user: DiscordUser = match http
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token.access_token)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(u) => u,
            Err(_) => return oauth_error_redirect("profile_fetch_failed"),
        },
        Err(_) => return oauth_error_redirect("profile_fetch_failed"),
        Ok(_) => return oauth_error_redirect("profile_fetch_failed"),
    };

    let Some(email) = discord_user.email else {
        return oauth_error_redirect("email_required");
    };

    // Build display name from Discord username
    let display_name = if discord_user.discriminator == "0" {
        // New Discord username system (no discriminator)
        discord_user.username.clone()
    } else {
        discord_user.username.clone()
    };

    let avatar_url = discord_user.avatar.as_ref().map(|hash| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=256",
            discord_user.id, hash
        )
    });

    let frontend = if is_desktop {
        "http://tauri.localhost".to_string()
    } else {
        frontend_base()
    };

    match issue_oauth_code(
        &state,
        &frontend,
        OAuthUserInfo {
            provider_id: discord_user.id,
            provider: "discord",
            email,
            username_hint: discord_user.username,
            display_name,
            avatar_url,
            provider_email_verified: discord_user.verified.unwrap_or(false),
        },
    )
    .await
    {
        Ok(response) => response,
        Err(_) => oauth_error_redirect("server_error"),
    }
}

// â”€â”€â”€ Google OAuth â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub async fn google_redirect(
    State(state): State<AppState>,
    Query(query): Query<OAuthRedirectQuery>,
) -> impl IntoResponse {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "missing".to_string());
    let redirect_uri = format!("{}/auth/oauth/google/callback", api_base());

    let csrf = new_state_token();
    let state_token = if query.is_desktop() {
        format!("{csrf}:desktop")
    } else {
        csrf
    };
    gc_oauth_states(&state.oauth_states);
    state
        .oauth_states
        .insert(state_token.clone(), Instant::now());

    let auth_url = url::Url::parse_with_params(
        "https://accounts.google.com/o/oauth2/v2/auth",
        &[
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "openid email profile"),
            ("state", state_token.as_str()),
            ("access_type", "online"),
        ],
    )
    .expect("Google auth URL is valid");

    (
        AppendHeaders([(header::SET_COOKIE, oauth_state_cookie_header(&state_token))]),
        Redirect::to(auth_url.as_str()),
    )
        .into_response()
}

pub async fn google_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<OAuthCallbackParams>,
) -> impl IntoResponse {
    // Validate CSRF state
    if !validate_oauth_state(&headers, &params.state, &state.oauth_states) {
        return oauth_error_redirect("invalid_state");
    }
    let is_desktop = params.state.ends_with(":desktop");

    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = format!("{}/auth/oauth/google/callback", api_base());

    let http = &state.http_client;

    // Exchange code for access token
    let token_res = match http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", params.code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Err(_) => return oauth_error_redirect("token_exchange_failed"),
        Ok(_) => return oauth_error_redirect("token_exchange_failed"),
    };

    #[derive(Deserialize)]
    struct GoogleTokenResponse {
        access_token: String,
    }

    let token: GoogleTokenResponse = match token_res.json().await {
        Ok(t) => t,
        Err(_) => return oauth_error_redirect("token_parse_failed"),
    };

    // Fetch Google user profile
    let google_user: GoogleUser = match http
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&token.access_token)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(u) => u,
            Err(_) => return oauth_error_redirect("profile_fetch_failed"),
        },
        Err(_) => return oauth_error_redirect("profile_fetch_failed"),
        Ok(_) => return oauth_error_redirect("profile_fetch_failed"),
    };

    let username_hint = google_user
        .email
        .split('@')
        .next()
        .unwrap_or("user")
        .to_string();

    let frontend = if is_desktop {
        "http://tauri.localhost".to_string()
    } else {
        frontend_base()
    };

    match issue_oauth_code(
        &state,
        &frontend,
        OAuthUserInfo {
            provider_id: google_user.id,
            provider: "google",
            email: google_user.email.clone(),
            username_hint,
            display_name: google_user.name,
            avatar_url: google_user.picture,
            provider_email_verified: google_user.verified_email.unwrap_or(false),
        },
    )
    .await
    {
        Ok(response) => response,
        Err(_) => oauth_error_redirect("server_error"),
    }
}

// â”€â”€â”€ Shared OAuth helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

struct OAuthUserInfo {
    provider_id: String,
    provider: &'static str,
    email: String,
    username_hint: String,
    display_name: String,
    avatar_url: Option<String>,
    provider_email_verified: bool,
}

pub struct ConsumedOAuthCode {
    pub user_id: Uuid,
}

pub(crate) async fn ensure_linked_identity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    provider: &str,
    provider_subject: &str,
) -> AppResult<()> {
    let existing_subject = sqlx::query_scalar::<_, String>(
        "SELECT provider_subject \
         FROM user_linked_identities \
         WHERE user_id = $1 AND provider = $2 \
         FOR UPDATE",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(existing_subject) = existing_subject {
        if existing_subject == provider_subject {
            return Ok(());
        }

        return Err(AppError::Conflict(format!(
            "This account already has a different {provider} identity linked"
        )));
    }

    sqlx::query(
        r#"
        INSERT INTO user_linked_identities (user_id, provider, provider_subject)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(provider)
    .bind(provider_subject)
    .execute(&mut **tx)
    .await
    .map_err(|error| {
        if is_unique_violation(&error) {
            AppError::Conflict("That OAuth account is already linked elsewhere".into())
        } else {
            AppError::Database(error)
        }
    })?;

    Ok(())
}

async fn issue_oauth_code(
    state: &AppState,
    frontend: &str,
    info: OAuthUserInfo,
) -> AppResult<axum::response::Response> {
    if !info.provider_email_verified {
        return Ok(oauth_error_redirect("email_unverified"));
    }

    // Sanitize username: keep alphanumeric + underscore, max 32 chars
    let clean_username = sanitize_username(&info.username_hint);
    let provider = info.provider;
    let provider_subject = info.provider_id.trim().to_string();
    let email = info.email.trim().to_lowercase();
    let pool = state.db.pool();
    let mut tx = pool.begin().await?;

    let existing_user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT uli.user_id
        FROM user_linked_identities uli
        JOIN users u ON u.id = uli.user_id
        WHERE uli.provider = $1
          AND uli.provider_subject = $2
          AND u.deleted_at IS NULL
        "#,
    )
    .bind(provider)
    .bind(&provider_subject)
    .fetch_optional(&mut *tx)
    .await?;

    let (user_id, is_new) = if let Some(user_id) = existing_user_id {
        if let Some(ref avatar) = info.avatar_url {
            sqlx::query("UPDATE users SET avatar_url = $1, last_seen_at = NOW() WHERE id = $2")
                .bind(avatar)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
        (user_id, false)
    } else {
        let by_email = sqlx::query(
            "SELECT id, email_verified FROM users WHERE email = $1 AND deleted_at IS NULL",
        )
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = by_email {
            let email_verified: bool = row.try_get("email_verified")?;
            if !email_verified {
                tx.rollback().await.ok();
                return Ok(oauth_error_redirect("email_verification_required"));
            }

            let user_id: Uuid = row.try_get("id")?;
            ensure_linked_identity(&mut tx, user_id, provider, &provider_subject).await?;

            if let Some(ref avatar) = info.avatar_url {
                sqlx::query("UPDATE users SET avatar_url = $1, last_seen_at = NOW() WHERE id = $2")
                    .bind(avatar)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            (user_id, false)
        } else {
            let username = unique_username(pool, &clean_username).await?;
            let row = sqlx::query(
                r#"
                INSERT INTO users
                    (email, email_verified, username, display_name, avatar_url, gdpr_consent_at)
                VALUES ($1, TRUE, $2, $3, $4, NOW())
                RETURNING id
                "#,
            )
            .bind(&email)
            .bind(&username)
            .bind(&info.display_name)
            .bind(&info.avatar_url)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| {
                if is_unique_violation(&error) {
                    AppError::Conflict("Email or username already taken".into())
                } else {
                    AppError::Database(error)
                }
            })?;

            let user_id: Uuid = row.try_get("id")?;
            ensure_linked_identity(&mut tx, user_id, provider, &provider_subject).await?;

            (user_id, true)
        }
    };

    let oauth_code = new_oauth_login_code();
    let code_hash = super::handlers::sha256_hex(&oauth_code);

    sqlx::query(
        r#"
        INSERT INTO oauth_login_codes (code_hash, user_id, provider, is_new, expires_at)
        VALUES ($1, $2, $3, $4, NOW() + INTERVAL '5 minutes')
        "#,
    )
    .bind(code_hash)
    .bind(user_id)
    .bind(provider)
    .bind(is_new)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let redirect_url = url::Url::parse_with_params(
        &format!("{}/oauth/callback", frontend),
        &[
            ("code", oauth_code.as_str()),
            ("is_new", if is_new { "true" } else { "false" }),
        ],
    )
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok((
        AppendHeaders([(header::SET_COOKIE, clear_oauth_state_cookie())]),
        Redirect::to(redirect_url.as_str()),
    )
        .into_response())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code == "23505")
        .unwrap_or(false)
}

pub async fn consume_oauth_login_code(
    state: &AppState,
    code: &str,
) -> AppResult<ConsumedOAuthCode> {
    let code_hash = super::handlers::sha256_hex(code);
    let row = sqlx::query!(
        r#"
        UPDATE oauth_login_codes
        SET consumed_at = NOW()
        WHERE code_hash = $1
          AND consumed_at IS NULL
          AND expires_at > NOW()
        RETURNING user_id, is_new
        "#,
        code_hash,
    )
    .fetch_optional(state.db.pool())
    .await?;

    let row = row.ok_or(AppError::Unauthorized)?;
    Ok(ConsumedOAuthCode {
        user_id: row.user_id,
    })
}

/// Make a username unique by appending random digits if needed.
async fn unique_username(pool: &sqlx::PgPool, base: &str) -> AppResult<String> {
    let candidate = base.to_string();
    let taken = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
        candidate
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);

    if !taken {
        return Ok(candidate);
    }

    // Try appending 4 random digits up to 10 times
    for _ in 0..10 {
        let mut bytes = [0u8; 2];
        getrandom::getrandom(&mut bytes).expect("getrandom failed to generate random bytes");
        let suffix = (u16::from_le_bytes(bytes) % 9000 + 1000) as u32;
        let with_suffix = format!("{base}{suffix}");
        let taken = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
            with_suffix
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(false);

        if !taken {
            return Ok(with_suffix);
        }
    }

    // Last resort: full UUID suffix
    Ok(format!(
        "{}_{}",
        base,
        &Uuid::new_v4().to_string().replace('-', "")[..8]
    ))
}

fn sanitize_username(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(28)
        .collect();

    if cleaned.len() < 2 {
        "user".to_string()
    } else {
        cleaned
    }
}

// â”€â”€â”€ Apple OAuth â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const APPLE_AUTH_URL: &str = "https://appleid.apple.com/auth/authorize";
const APPLE_TOKEN_URL: &str = "https://appleid.apple.com/auth/token";
const APPLE_JWKS_URL: &str = "https://appleid.apple.com/auth/keys";

#[derive(Debug, Deserialize)]
struct AppleIdTokenClaims {
    sub: String,
    email: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    email_verified: Option<bool>,
}

#[derive(serde::Serialize)]
struct AppleClientSecretClaims {
    iss: String,
    aud: String,
    sub: String,
    iat: i64,
    exp: i64,
}

fn generate_apple_client_secret() -> anyhow::Result<String> {
    let team_id =
        std::env::var("APPLE_TEAM_ID").map_err(|_| anyhow::anyhow!("APPLE_TEAM_ID not set"))?;
    let key_id =
        std::env::var("APPLE_KEY_ID").map_err(|_| anyhow::anyhow!("APPLE_KEY_ID not set"))?;
    let client_id =
        std::env::var("APPLE_CLIENT_ID").map_err(|_| anyhow::anyhow!("APPLE_CLIENT_ID not set"))?;
    // APPLE_PRIVATE_KEY: contents of the .p8 file (PKCS#8 PEM, -----BEGIN PRIVATE KEY-----)
    let private_key_pem = std::env::var("APPLE_PRIVATE_KEY")
        .map_err(|_| anyhow::anyhow!("APPLE_PRIVATE_KEY not set"))?;

    let now = Utc::now().timestamp();
    let claims = AppleClientSecretClaims {
        iss: team_id,
        aud: "https://appleid.apple.com".to_string(),
        sub: client_id,
        iat: now,
        exp: now + 15_777_000, // ~6 months
    };

    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
    header.kid = Some(key_id);

    let key = jsonwebtoken::EncodingKey::from_ec_pem(private_key_pem.as_bytes())?;
    Ok(jsonwebtoken::encode(&header, &claims, &key)?)
}

pub async fn apple_redirect(State(state): State<AppState>) -> impl IntoResponse {
    let client_id = std::env::var("APPLE_CLIENT_ID").unwrap_or_else(|_| "missing".to_string());
    let redirect_uri = format!("{}/auth/oauth/apple/callback", api_base());

    let csrf = new_state_token();
    gc_oauth_states(&state.oauth_states);
    state.oauth_states.insert(csrf.clone(), Instant::now());

    let auth_url = url::Url::parse_with_params(
        APPLE_AUTH_URL,
        &[
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "name email"),
            ("response_mode", "form_post"),
            ("state", csrf.as_str()),
        ],
    )
    .expect("Apple auth URL is valid");

    (
        AppendHeaders([(header::SET_COOKIE, oauth_state_cookie_header(&csrf))]),
        Redirect::to(auth_url.as_str()),
    )
        .into_response()
}

/// Apple sends callback as POST form_post (not a GET redirect like Discord/Google).
#[derive(Debug, Deserialize)]
pub struct AppleCallbackForm {
    code: Option<String>,
    state: Option<String>,
    id_token: Option<String>,
    /// JSON string with name info â€” only present on the VERY FIRST authorization.
    user: Option<String>,
    error: Option<String>,
}

pub async fn apple_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<AppleCallbackForm>,
) -> impl IntoResponse {
    if form.error.is_some() {
        return oauth_error_redirect("apple_denied");
    }

    let code = match form.code {
        Some(c) => c,
        None => return oauth_error_redirect("missing_code"),
    };
    let csrf_state = match form.state {
        Some(s) => s,
        None => return oauth_error_redirect("missing_state"),
    };

    if !validate_oauth_state(&headers, &csrf_state, &state.oauth_states) {
        return oauth_error_redirect("invalid_state");
    }

    let client_id = std::env::var("APPLE_CLIENT_ID").unwrap_or_default();
    let redirect_uri = format!("{}/auth/oauth/apple/callback", api_base());

    let client_secret = match generate_apple_client_secret() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Apple client secret generation failed: {e}");
            return oauth_error_redirect("server_error");
        }
    };

    let http = &state.http_client;

    let token_res = match http
        .post(APPLE_TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return oauth_error_redirect("token_exchange_failed"),
    };

    #[derive(Deserialize)]
    struct AppleTokenResponse {
        id_token: Option<String>,
    }

    if !token_res.status().is_success() {
        return oauth_error_redirect("token_exchange_failed");
    }

    let token_data: AppleTokenResponse = match token_res.json().await {
        Ok(t) => t,
        Err(_) => return oauth_error_redirect("token_parse_failed"),
    };

    // Prefer id_token from the token exchange; fall back to the one Apple POSTed directly.
    let id_token_str = match token_data.id_token.or(form.id_token) {
        Some(t) => t,
        None => return oauth_error_redirect("no_id_token"),
    };

    let claims = match verify_apple_id_token(http, &id_token_str, &client_id).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Apple ID token verification failed: {e}");
            return oauth_error_redirect("id_token_invalid");
        }
    };

    let email = match claims.email {
        Some(e) => e,
        None => return oauth_error_redirect("email_required"),
    };

    // Parse display name from 'user' JSON (only on first sign-in).
    let display_name = form
        .user
        .as_deref()
        .and_then(|u| serde_json::from_str::<serde_json::Value>(u).ok())
        .and_then(|v| {
            let first = v["name"]["firstName"].as_str().unwrap_or("").to_string();
            let last = v["name"]["lastName"].as_str().unwrap_or("").to_string();
            let name = format!("{first} {last}").trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        })
        .unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());

    let username_hint = email.split('@').next().unwrap_or("user").to_string();

    match issue_oauth_code(
        &state,
        &frontend_base(),
        OAuthUserInfo {
            provider_id: claims.sub,
            provider: "apple",
            email,
            username_hint,
            display_name,
            avatar_url: None, // Apple does not provide avatars
            provider_email_verified: claims.email_verified.unwrap_or(false),
        },
    )
    .await
    {
        Ok(response) => response,
        Err(_) => oauth_error_redirect("server_error"),
    }
}

async fn verify_apple_id_token(
    http: &reqwest::Client,
    id_token: &str,
    client_id: &str,
) -> anyhow::Result<AppleIdTokenClaims> {
    // Fetch Apple's public JWKS
    let jwks: serde_json::Value = http.get(APPLE_JWKS_URL).send().await?.json().await?;

    // Determine which key to use from the token header
    let header = jsonwebtoken::decode_header(id_token)?;
    let kid = header
        .kid
        .ok_or_else(|| anyhow::anyhow!("no kid in Apple ID token header"))?;

    let keys = jwks["keys"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("JWKS missing keys array"))?;

    let jwk_val = keys
        .iter()
        .find(|k| k["kid"].as_str() == Some(&kid))
        .ok_or_else(|| anyhow::anyhow!("no matching JWK for kid={kid}"))?;

    let jwk: jsonwebtoken::jwk::Jwk = serde_json::from_value(jwk_val.clone())?;
    let decoding_key = jsonwebtoken::DecodingKey::from_jwk(&jwk)?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&["https://appleid.apple.com"]);

    let token_data =
        jsonwebtoken::decode::<AppleIdTokenClaims>(id_token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

fn deserialize_optional_boolish<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Bool(value)) => Some(value),
        Some(serde_json::Value::String(value)) => match value.as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

fn oauth_error_redirect(reason: &str) -> axum::response::Response {
    let url = url::Url::parse_with_params(
        &format!("{}/login", frontend_base()),
        &[("oauth_error", reason)],
    )
    .expect("OAuth error redirect is valid");
    (
        AppendHeaders([(header::SET_COOKIE, clear_oauth_state_cookie())]),
        Redirect::to(url.as_str()),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::ensure_linked_identity;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{PgPool, Row};
    use uuid::Uuid;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .ok()?;
        if sqlx::migrate!("./migrations").run(&pool).await.is_err() {
            return None;
        }
        Some(pool)
    }

    async fn insert_user(pool: &PgPool, suffix: &str) -> Uuid {
        let row = sqlx::query(
            "INSERT INTO users (email, username, display_name, account_type, parental_controls_enabled) \
             VALUES ($1, $2, $3, 'standard', FALSE) RETURNING id",
        )
        .bind(format!("oauth_link_{suffix}@example.com"))
        .bind(format!("oauth_link_{suffix}"))
        .bind(format!("OAuth Link {suffix}"))
        .fetch_one(pool)
        .await
        .expect("insert user");
        row.try_get("id").expect("id")
    }

    #[tokio::test]
    async fn linked_identity_allows_same_subject_reuse() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
        let user_id = insert_user(&pool, &suffix).await;
        let mut tx = pool.begin().await.expect("begin");

        ensure_linked_identity(&mut tx, user_id, "google", "same-subject")
            .await
            .expect("initial link");
        ensure_linked_identity(&mut tx, user_id, "google", "same-subject")
            .await
            .expect("same subject should be idempotent");

        tx.rollback().await.expect("rollback");
    }

    #[tokio::test]
    async fn linked_identity_rejects_subject_mismatch_for_same_provider() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let suffix = Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
        let user_id = insert_user(&pool, &suffix).await;
        let mut tx = pool.begin().await.expect("begin");

        ensure_linked_identity(&mut tx, user_id, "discord", "discord-old")
            .await
            .expect("initial link");
        let err = ensure_linked_identity(&mut tx, user_id, "discord", "discord-new")
            .await
            .expect_err("subject mismatch should conflict");

        assert!(matches!(err, crate::error::AppError::Conflict(_)));
        tx.rollback().await.expect("rollback");
    }
}
