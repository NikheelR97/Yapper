use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// HIGH-001: Limit concurrent Argon2 operations to prevent OOM on 256MB VM.
/// Each Argon2id hash requires 64MB (m=65536). With 2 permits, peak memory
/// usage is 128MB — safe headroom on a 256MB Fly.io VM.
static ARGON2_SEMAPHORE: once_cell::sync::Lazy<Semaphore> =
    once_cell::sync::Lazy::new(|| Semaphore::new(2));

// ─── JWT Claims ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessClaims {
    /// Subject: user UUID
    pub sub: Uuid,
    /// Expiry (Unix timestamp)
    pub exp: i64,
    /// Issued-at
    pub iat: i64,
    /// Key ID (for rotation)
    pub kid: String,
    /// Account type: standard | parent | child | bot
    pub account_type: String,
    /// Authenticated device UUID for v2 device-bound sessions.
    #[serde(default)]
    pub device_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    /// Token family ID — all rotations of a session share this
    pub family_id: Uuid,
    /// Unique ID for this specific refresh token
    pub jti: Uuid,
    /// Authenticated device UUID for v2 device-bound sessions.
    #[serde(default)]
    pub device_id: Option<Uuid>,
}

/// Access token TTL: 15 minutes
pub const ACCESS_TTL_SECS: i64 = 900;
/// Refresh token TTL: 30 days
pub const REFRESH_TTL_SECS: i64 = 30 * 24 * 3600;

// ─── Keys ────────────────────────────────────────────────────────────────────

pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
    pub kid: String,
}

impl JwtKeys {
    /// Load RS256 keys from PEM environment variables.
    /// In production, keys are stored as Fly.io secrets:
    ///   fly secrets set JWT_PRIVATE_KEY="$(cat jwt_private.pem)"
    ///   fly secrets set JWT_PUBLIC_KEY="$(cat jwt_public.pem)"
    pub fn from_env() -> anyhow::Result<Self> {
        let private_pem = std::env::var("JWT_PRIVATE_KEY")
            .or_else(|_| {
                // Fallback: read from file path (development)
                let path = std::env::var("JWT_PRIVATE_KEY_PATH")
                    .unwrap_or_else(|_| "./jwt_private.pem".to_string());
                std::fs::read_to_string(path)
            })
            .map_err(|_| anyhow::anyhow!("JWT_PRIVATE_KEY or JWT_PRIVATE_KEY_PATH must be set"))?;

        let public_pem = std::env::var("JWT_PUBLIC_KEY")
            .or_else(|_| {
                let path = std::env::var("JWT_PUBLIC_KEY_PATH")
                    .unwrap_or_else(|_| "./jwt_public.pem".to_string());
                std::fs::read_to_string(path)
            })
            .map_err(|_| anyhow::anyhow!("JWT_PUBLIC_KEY or JWT_PUBLIC_KEY_PATH must be set"))?;

        let kid = std::env::var("JWT_KEY_ID").unwrap_or_else(|_| "v1".to_string());

        Ok(Self {
            encoding: EncodingKey::from_rsa_pem(private_pem.as_bytes())?,
            decoding: DecodingKey::from_rsa_pem(public_pem.as_bytes())?,
            kid,
        })
    }
}

// ─── Password ─────────────────────────────────────────────────────────────────

/// Argon2id parameters: m=65536 (64MB), t=3 iterations, p=4 lanes
pub fn hash_password(password: &str) -> AppResult<String> {
    // Reject passwords over 1KB — prevents DOS via Argon2 with huge inputs
    if password.len() > 1024 {
        return Err(AppError::BadRequest("Password too long".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(65536, 3, 4, None)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Argon2 params: {e}")))?,
    );

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Hash error: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    if password.len() > 1024 {
        return Ok(false);
    }

    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Hash parse: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Async wrapper that runs Argon2 verification on the blocking thread pool,
/// preventing it from stalling the Tokio runtime (~200-500ms per call).
///
/// HIGH-001: Acquires a semaphore permit before hashing to limit concurrent
/// Argon2 operations (64MB each) and prevent OOM on constrained VMs.
pub async fn verify_password_async(password: String, hash: String) -> AppResult<bool> {
    let _permit = ARGON2_SEMAPHORE
        .acquire()
        .await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Argon2 semaphore closed")))?;
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("spawn_blocking join: {e}")))?
}

/// Async wrapper for password hashing on the blocking thread pool.
///
/// HIGH-001: Acquires a semaphore permit before hashing to limit concurrent
/// Argon2 operations (64MB each) and prevent OOM on constrained VMs.
pub async fn hash_password_async(password: String) -> AppResult<String> {
    let _permit = ARGON2_SEMAPHORE
        .acquire()
        .await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Argon2 semaphore closed")))?;
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("spawn_blocking join: {e}")))?
}

// ─── Token generation ─────────────────────────────────────────────────────────

pub fn generate_access_token(
    user_id: Uuid,
    account_type: &str,
    device_id: Option<Uuid>,
    keys: &JwtKeys,
) -> AppResult<String> {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        exp: (now + Duration::seconds(ACCESS_TTL_SECS)).timestamp(),
        iat: now.timestamp(),
        kid: keys.kid.clone(),
        account_type: account_type.to_string(),
        device_id,
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(keys.kid.clone());

    encode(&header, &claims, &keys.encoding)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode: {e}")))
}

pub fn generate_refresh_token(
    user_id: Uuid,
    family_id: Uuid,
    device_id: Option<Uuid>,
    keys: &JwtKeys,
) -> AppResult<String> {
    let now = Utc::now();
    let claims = RefreshClaims {
        sub: user_id,
        exp: (now + Duration::seconds(REFRESH_TTL_SECS)).timestamp(),
        iat: now.timestamp(),
        family_id,
        jti: Uuid::new_v4(),
        device_id,
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(keys.kid.clone());

    encode(&header, &claims, &keys.encoding)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode: {e}")))
}

pub fn validate_access_token(token: &str, keys: &JwtKeys) -> AppResult<TokenData<AccessClaims>> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;

    decode::<AccessClaims>(token, &keys.decoding, &validation).map_err(|_| AppError::Unauthorized)
}

pub fn validate_refresh_token(token: &str, keys: &JwtKeys) -> AppResult<TokenData<RefreshClaims>> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;

    decode::<RefreshClaims>(token, &keys.decoding, &validation).map_err(|_| AppError::Unauthorized)
}

// ─── Email verification token ─────────────────────────────────────────────────

/// Generate a cryptographically random hex token for email verification / password reset.
pub fn generate_email_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    crate::hex_encode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_password_hash_and_verify() {
        let password = Uuid::new_v4().to_string();
        let wrong_password = Uuid::new_v4().to_string();
        let hash = hash_password(&password).unwrap();
        assert!(verify_password(&password, &hash).unwrap());
        assert!(!verify_password(&wrong_password, &hash).unwrap());
    }

    #[test]
    fn test_password_too_long_rejected() {
        let long_pw = "a".repeat(1025);
        assert!(hash_password(&long_pw).is_err());
    }

    #[test]
    fn test_email_token_is_64_hex_chars() {
        let token = generate_email_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_email_tokens_are_unique() {
        let t1 = generate_email_token();
        let t2 = generate_email_token();
        assert_ne!(t1, t2);
    }
}
