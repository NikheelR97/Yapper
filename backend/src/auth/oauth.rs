use axum::{
    extract::{Form, Query, State},
    http::header,
    response::{AppendHeaders, IntoResponse, Redirect},
};
use chrono::Utc;
use serde::Deserialize;
use std::time::Instant;
use uuid::Uuid;

use super::service::{generate_access_token, generate_refresh_token, REFRESH_TTL_SECS};
use crate::{error::AppResult, AppState};

// ─── Discord user info ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    discriminator: String,
    email: Option<String>,
    avatar: Option<String>,
}

// ─── Google user info ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GoogleUser {
    id: String,
    email: String,
    name: String,
    picture: Option<String>,
}

// ─── OAuth state store ────────────────────────────────────────────────────────

/// Short-lived state tokens keyed by the random state value.
pub type OAuthStateStore = dashmap::DashMap<String, Instant>;

fn api_base() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

fn frontend_base() -> String {
    std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string())
}

fn new_state_token() -> String {
    use std::fmt::Write;
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        write!(s, "{:02x}", b).unwrap();
        s
    })
}

/// Remove state tokens older than 10 minutes to avoid unbounded growth.
fn gc_oauth_states(store: &OAuthStateStore) {
    let cutoff = Instant::now();
    store.retain(|_, created_at| cutoff.duration_since(*created_at).as_secs() < 600);
}

// ─── Callback query params ────────────────────────────────────────────────────

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

// ─── Discord OAuth ─────────────────────────────────────────────────────────────

pub async fn discord_redirect(State(state): State<AppState>) -> impl IntoResponse {
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_else(|_| "missing".to_string());
    let redirect_uri = format!("{}/auth/oauth/discord/callback", api_base());

    let csrf = new_state_token();
    gc_oauth_states(&state.oauth_states);
    state.oauth_states.insert(csrf.clone(), Instant::now());

    let auth_url = url::Url::parse_with_params(
        "https://discord.com/api/oauth2/authorize",
        &[
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "identify email"),
            ("state", csrf.as_str()),
        ],
    )
    .expect("Discord auth URL is valid");

    Redirect::to(auth_url.as_str())
}

pub async fn discord_callback(
    State(state): State<AppState>,
    Query(params): Query<OAuthCallbackParams>,
) -> impl IntoResponse {
    // Validate CSRF state
    if state.oauth_states.remove(&params.state).is_none() {
        return oauth_error_redirect("invalid_state");
    }

    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("DISCORD_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = format!("{}/auth/oauth/discord/callback", api_base());

    // Exchange code for access token
    let http = reqwest::Client::new();
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
        Ok(r) => r,
        Err(_) => return oauth_error_redirect("token_exchange_failed"),
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
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(_) => return oauth_error_redirect("profile_fetch_failed"),
        },
        Err(_) => return oauth_error_redirect("profile_fetch_failed"),
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

    match issue_oauth_session(
        &state,
        OAuthUserInfo {
            provider_id: discord_user.id,
            provider: "discord",
            email,
            username_hint: discord_user.username,
            display_name,
            avatar_url,
        },
    )
    .await
    {
        Ok(response) => response,
        Err(_) => oauth_error_redirect("server_error"),
    }
}

// ─── Google OAuth ─────────────────────────────────────────────────────────────

pub async fn google_redirect(State(state): State<AppState>) -> impl IntoResponse {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "missing".to_string());
    let redirect_uri = format!("{}/auth/oauth/google/callback", api_base());

    let csrf = new_state_token();
    gc_oauth_states(&state.oauth_states);
    state.oauth_states.insert(csrf.clone(), Instant::now());

    let auth_url = url::Url::parse_with_params(
        "https://accounts.google.com/o/oauth2/v2/auth",
        &[
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "openid email profile"),
            ("state", csrf.as_str()),
            ("access_type", "online"),
        ],
    )
    .expect("Google auth URL is valid");

    Redirect::to(auth_url.as_str())
}

pub async fn google_callback(
    State(state): State<AppState>,
    Query(params): Query<OAuthCallbackParams>,
) -> impl IntoResponse {
    // Validate CSRF state
    if state.oauth_states.remove(&params.state).is_none() {
        return oauth_error_redirect("invalid_state");
    }

    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = format!("{}/auth/oauth/google/callback", api_base());

    let http = reqwest::Client::new();

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
        Ok(r) => r,
        Err(_) => return oauth_error_redirect("token_exchange_failed"),
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
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(_) => return oauth_error_redirect("profile_fetch_failed"),
        },
        Err(_) => return oauth_error_redirect("profile_fetch_failed"),
    };

    let username_hint = google_user
        .email
        .split('@')
        .next()
        .unwrap_or("user")
        .to_string();

    match issue_oauth_session(
        &state,
        OAuthUserInfo {
            provider_id: google_user.id,
            provider: "google",
            email: google_user.email.clone(),
            username_hint,
            display_name: google_user.name,
            avatar_url: google_user.picture,
        },
    )
    .await
    {
        Ok(response) => response,
        Err(_) => oauth_error_redirect("server_error"),
    }
}

// ─── Shared OAuth helpers ─────────────────────────────────────────────────────

struct OAuthUserInfo {
    provider_id: String,
    provider: &'static str,
    email: String,
    username_hint: String,
    display_name: String,
    avatar_url: Option<String>,
}

/// User DTO for OAuth queries.
#[derive(sqlx::FromRow)]
struct OAuthUserRow {
    id: Uuid,
    account_type: String,
    #[allow(dead_code)]
    is_new: bool,
}

async fn issue_oauth_session(
    state: &AppState,
    info: OAuthUserInfo,
) -> AppResult<axum::response::Response> {
    let pool = state.db.pool();
    let email = info.email.to_lowercase();

    // Sanitize username: keep alphanumeric + underscore, max 32 chars
    let clean_username = sanitize_username(&info.username_hint);

    // Upsert user: find by discord_id/google_id (stored in discord_id for discord,
    // or by email for Google), or create new.
    // We use a single discord_id column for all OAuth provider IDs prefixed by provider.
    let provider_key = format!("{}:{}", info.provider, info.provider_id);

    // Try find by provider key (stored in discord_id column for now)
    let existing = sqlx::query_as!(
        OAuthUserRow,
        r#"
        SELECT id, account_type, FALSE AS "is_new!"
        FROM users
        WHERE discord_id = $1 AND deleted_at IS NULL
        "#,
        provider_key,
    )
    .fetch_optional(pool)
    .await?;

    let (user_id, account_type, is_new) = if let Some(u) = existing {
        // Known user — update avatar/display_name if changed
        if let Some(ref avatar) = info.avatar_url {
            sqlx::query!(
                "UPDATE users SET avatar_url = $1, last_seen_at = NOW() WHERE id = $2",
                avatar,
                u.id,
            )
            .execute(pool)
            .await?;
        }
        (u.id, u.account_type, false)
    } else {
        // Try find by email (link existing account)
        let by_email = sqlx::query!(
            "SELECT id, account_type FROM users WHERE email = $1 AND deleted_at IS NULL",
            email,
        )
        .fetch_optional(pool)
        .await?;

        if let Some(u) = by_email {
            // Link OAuth to existing email account
            sqlx::query!(
                "UPDATE users SET discord_id = $1, last_seen_at = NOW() WHERE id = $2",
                provider_key,
                u.id,
            )
            .execute(pool)
            .await?;
            (u.id, u.account_type, false)
        } else {
            // Create new user
            let username = unique_username(pool, &clean_username).await?;
            let user = sqlx::query!(
                r#"
                INSERT INTO users
                    (email, email_verified, username, display_name, avatar_url,
                     discord_id, gdpr_consent_at)
                VALUES ($1, TRUE, $2, $3, $4, $5, NOW())
                RETURNING id, account_type
                "#,
                email,
                username,
                info.display_name,
                info.avatar_url,
                provider_key,
            )
            .fetch_one(pool)
            .await?;
            (user.id, user.account_type, true)
        }
    };

    let access_token = generate_access_token(user_id, &account_type, &state.jwt_keys)?;
    let refresh_token = generate_refresh_token(user_id, Uuid::new_v4(), &state.jwt_keys)?;

    // Store session
    let claims = super::service::validate_refresh_token(&refresh_token, &state.jwt_keys)?.claims;
    let token_hash = sha256_hex(&refresh_token);
    let expires_at = chrono::DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(Utc::now);

    sqlx::query!(
        "INSERT INTO sessions (user_id, family_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        user_id,
        claims.family_id,
        token_hash,
        expires_at,
    )
    .execute(pool)
    .await?;

    let use_secure_cookie = crate::csrf::should_use_secure_cookie();
    let secure_flag = if use_secure_cookie { "; Secure" } else { "" };
    let same_site = if use_secure_cookie { "None" } else { "Lax" };
    let refresh_cookie = format!(
        "refresh_token={refresh_token}; HttpOnly{secure_flag}; SameSite={same_site}; Path=/auth/oauth; Max-Age={REFRESH_TTL_SECS}"
    );

    // Generate a CSRF token so that state-mutating POSTs (e.g. Signal key upload)
    // work immediately after OAuth login — same as the regular /auth/login flow.
    // Also returned as a URL param so the cross-origin frontend (Tauri/Capacitor) can read it.
    let csrf_token = new_state_token();
    let csrf_cookie = crate::csrf::csrf_cookie_header(&csrf_token);

    let redirect_url = format!(
        "{}/oauth/callback?access_token={}&is_new={}&csrf_token={}",
        frontend_base(),
        access_token,
        is_new,
        csrf_token,
    );

    Ok((
        AppendHeaders([
            (header::SET_COOKIE, refresh_cookie),
            (header::SET_COOKIE, csrf_cookie),
        ]),
        Redirect::to(&redirect_url),
    )
        .into_response())
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
        getrandom::getrandom(&mut bytes).unwrap();
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

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ─── Apple OAuth ──────────────────────────────────────────────────────────────

const APPLE_AUTH_URL: &str = "https://appleid.apple.com/auth/authorize";
const APPLE_TOKEN_URL: &str = "https://appleid.apple.com/auth/token";
const APPLE_JWKS_URL: &str = "https://appleid.apple.com/auth/keys";

#[derive(Debug, Deserialize)]
struct AppleIdTokenClaims {
    sub: String,
    email: Option<String>,
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

    Redirect::to(auth_url.as_str())
}

/// Apple sends callback as POST form_post (not a GET redirect like Discord/Google).
#[derive(Debug, Deserialize)]
pub struct AppleCallbackForm {
    code: Option<String>,
    state: Option<String>,
    id_token: Option<String>,
    /// JSON string with name info — only present on the VERY FIRST authorization.
    user: Option<String>,
    error: Option<String>,
}

pub async fn apple_callback(
    State(state): State<AppState>,
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

    if state.oauth_states.remove(&csrf_state).is_none() {
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

    let http = reqwest::Client::new();

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

    let token_data: AppleTokenResponse = match token_res.json().await {
        Ok(t) => t,
        Err(_) => return oauth_error_redirect("token_parse_failed"),
    };

    // Prefer id_token from the token exchange; fall back to the one Apple POSTed directly.
    let id_token_str = match token_data.id_token.or(form.id_token) {
        Some(t) => t,
        None => return oauth_error_redirect("no_id_token"),
    };

    let claims = match verify_apple_id_token(&http, &id_token_str, &client_id).await {
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

    match issue_oauth_session(
        &state,
        OAuthUserInfo {
            provider_id: claims.sub,
            provider: "apple",
            email,
            username_hint,
            display_name,
            avatar_url: None, // Apple does not provide avatars
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

fn oauth_error_redirect(reason: &str) -> axum::response::Response {
    let url = format!("{}/login?oauth_error={}", frontend_base(), reason);
    Redirect::to(&url).into_response()
}
