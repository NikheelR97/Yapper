use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use serde_json::Value;
use sha2::Sha256;
use sqlx::Row;
use std::num::NonZeroU32;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

const MAX_PROMO_CODE_LENGTH: usize = 64;
const PROMO_ATTEMPTS_PER_HOUR: u32 = 5;
const MAX_STRIPE_WEBHOOK_BODY_BYTES: usize = 64 * 1024;
const STRIPE_TOLERANCE_SECS: i64 = 300;

const GOPRO_FEATURES: &[&str] = &[
    "Animated avatars",
    "100 custom emoji slots per server (vs 50 free)",
    "Full hex profile theme picker",
    "1080p + 4K video clips (vs 720p)",
    "30-minute yap recording (vs 5 minutes)",
    "Server boosted audio/video quality",
    "Priority support",
    "GoPro badge on your profile",
];

type PromoRateLimiter = RateLimiter<Uuid, DefaultKeyedStateStore<Uuid>, DefaultClock>;

static PROMO_RATE_LIMITER: Lazy<PromoRateLimiter> = Lazy::new(|| {
    let hourly_quota =
        Quota::per_hour(NonZeroU32::new(PROMO_ATTEMPTS_PER_HOUR).expect("non-zero quota"));
    RateLimiter::keyed(hourly_quota)
});

#[derive(Debug, Clone)]
pub struct PremiumStatus {
    pub is_premium: bool,
    pub premium_since: Option<DateTime<Utc>>,
}

impl PremiumStatus {
    pub fn plan(&self) -> &'static str {
        if self.is_premium {
            "gopro"
        } else {
            "free"
        }
    }
}

pub fn gopro_features() -> &'static [&'static str] {
    GOPRO_FEATURES
}

pub fn check_promo_rate_limit(user_id: Uuid) -> AppResult<()> {
    if PROMO_RATE_LIMITER.check_key(&user_id).is_ok() {
        return Ok(());
    }
    Err(AppError::RateLimited)
}

pub fn normalize_promo_code(code: &str) -> AppResult<String> {
    let normalized = code.trim().to_uppercase();
    if normalized.is_empty() {
        return Err(AppError::BadRequest("Promo code is required".into()));
    }
    if normalized.len() > MAX_PROMO_CODE_LENGTH {
        return Err(AppError::BadRequest(format!(
            "Promo code must be <= {MAX_PROMO_CODE_LENGTH} characters"
        )));
    }
    Ok(normalized)
}

pub async fn load_status(user_id: Uuid, state: &AppState) -> AppResult<PremiumStatus> {
    let row = sqlx::query(
        "SELECT is_premium, premium_since FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?;

    let Some(row) = row else {
        return Err(AppError::NotFound("User not found".into()));
    };

    Ok(PremiumStatus {
        is_premium: row.try_get::<bool, _>("is_premium").unwrap_or(false),
        premium_since: row
            .try_get::<Option<DateTime<Utc>>, _>("premium_since")
            .ok()
            .flatten(),
    })
}

pub async fn activate_with_promo(
    user_id: Uuid,
    promo_code: &str,
    state: &AppState,
) -> AppResult<PremiumStatus> {
    let valid_codes = configured_promo_codes();
    if valid_codes.is_empty() {
        return Err(AppError::BadRequest(
            "Promo code activation is not enabled on this server".into(),
        ));
    }
    if !valid_codes.contains(&promo_code.to_string()) {
        return Err(AppError::BadRequest("Invalid promo code".into()));
    }

    let row = sqlx::query(
        r#"
        UPDATE users
        SET is_premium = TRUE, premium_since = NOW()
        WHERE id = $1 AND deleted_at IS NULL AND is_premium = FALSE
        RETURNING is_premium, premium_since
        "#,
    )
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?;

    if let Some(updated) = row {
        return Ok(PremiumStatus {
            is_premium: updated.try_get::<bool, _>("is_premium").unwrap_or(true),
            premium_since: updated
                .try_get::<Option<DateTime<Utc>>, _>("premium_since")
                .ok()
                .flatten(),
        });
    }

    let current = load_status(user_id, state).await?;
    if current.is_premium {
        return Err(AppError::BadRequest("Your account is already GoPro".into()));
    }

    Err(AppError::NotFound("User not found".into()))
}

pub async fn disable_local_access(user_id: Uuid, state: &AppState) -> AppResult<PremiumStatus> {
    let row = sqlx::query(
        r#"
        UPDATE users
        SET is_premium = FALSE, premium_since = NULL
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING is_premium, premium_since
        "#,
    )
    .bind(user_id)
    .fetch_optional(state.db.pool())
    .await?;

    let Some(updated) = row else {
        return Err(AppError::NotFound("User not found".into()));
    };

    Ok(PremiumStatus {
        is_premium: updated.try_get::<bool, _>("is_premium").unwrap_or(false),
        premium_since: updated
            .try_get::<Option<DateTime<Utc>>, _>("premium_since")
            .ok()
            .flatten(),
    })
}

pub async fn process_stripe_webhook(
    headers: &HeaderMap,
    body: &Bytes,
    secret: &str,
    state: &AppState,
) -> StatusCode {
    if !is_valid_webhook_body_size(body.len()) {
        return StatusCode::PAYLOAD_TOO_LARGE;
    }

    let sig_header = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(value) => value,
        None => return StatusCode::BAD_REQUEST,
    };

    if !verify_stripe_signature(sig_header, body, secret) {
        tracing::warn!("Stripe webhook signature verification failed");
        return StatusCode::UNAUTHORIZED;
    }

    let event: Value = match serde_json::from_slice(body) {
        Ok(payload) => payload,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    process_verified_event(&event, state).await
}

fn configured_promo_codes() -> Vec<String> {
    std::env::var("GOPRO_PROMO_CODES")
        .unwrap_or_default()
        .split(',')
        .map(|raw| raw.trim().to_uppercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn extract_user_id(obj: &Value) -> Option<Uuid> {
    obj["metadata"]["user_id"]
        .as_str()
        .and_then(|value| value.parse::<Uuid>().ok())
}

fn should_be_premium(subscription_status: &str) -> bool {
    matches!(subscription_status, "active" | "trialing")
}

fn is_paid_checkout(obj: &Value) -> bool {
    let payment_status = obj["payment_status"].as_str().unwrap_or_default();
    matches!(payment_status, "paid" | "no_payment_required")
}

fn is_valid_webhook_body_size(body_len: usize) -> bool {
    if body_len <= MAX_STRIPE_WEBHOOK_BODY_BYTES {
        return true;
    }

    tracing::warn!(
        body_size = body_len,
        max_size = MAX_STRIPE_WEBHOOK_BODY_BYTES,
        "Stripe webhook rejected: body too large"
    );
    false
}

async fn process_verified_event(event: &Value, state: &AppState) -> StatusCode {
    let event_type = event["type"].as_str().unwrap_or("");
    let obj = &event["data"]["object"];
    let uid = match extract_user_id(obj) {
        Some(user_id) => user_id,
        None => {
            tracing::warn!(
                event = event_type,
                "Stripe webhook missing metadata.user_id"
            );
            return StatusCode::BAD_REQUEST;
        }
    };

    let result = match event_type {
        "checkout.session.completed" => handle_checkout_completed(uid, obj, state).await,
        "customer.subscription.updated" => handle_subscription_updated(uid, obj, state).await,
        "customer.subscription.deleted" => update_entitlement(uid, false, state).await,
        _ => return StatusCode::OK,
    };

    match result {
        Ok(()) => StatusCode::OK,
        Err(error) => {
            tracing::error!(
                error = %error,
                user_id = %uid,
                event = event_type,
                "Stripe webhook DB update failed"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn handle_checkout_completed(
    user_id: Uuid,
    obj: &Value,
    state: &AppState,
) -> Result<(), sqlx::Error> {
    if !is_paid_checkout(obj) {
        tracing::info!(
            user_id = %user_id,
            "Ignoring unpaid checkout.session.completed event"
        );
        return Ok(());
    }

    update_entitlement(user_id, true, state).await
}

async fn handle_subscription_updated(
    user_id: Uuid,
    obj: &Value,
    state: &AppState,
) -> Result<(), sqlx::Error> {
    let status = obj["status"].as_str().unwrap_or_default();
    update_entitlement(user_id, should_be_premium(status), state).await
}

async fn update_entitlement(
    user_id: Uuid,
    is_premium: bool,
    state: &AppState,
) -> Result<(), sqlx::Error> {
    let premium_since: Option<DateTime<Utc>> = if is_premium { Some(Utc::now()) } else { None };

    sqlx::query(
        "UPDATE users SET is_premium = $1, premium_since = $2 WHERE id = $3 AND deleted_at IS NULL",
    )
    .bind(is_premium)
    .bind(premium_since)
    .bind(user_id)
    .execute(state.db.pool())
    .await?;

    Ok(())
}

fn verify_stripe_signature(sig_header: &str, body: &[u8], secret: &str) -> bool {
    verify_stripe_signature_at(sig_header, body, secret, Utc::now().timestamp())
}

fn verify_stripe_signature_at(sig_header: &str, body: &[u8], secret: &str, now_ts: i64) -> bool {
    let (timestamp, expected_sig) = match parse_signature_header(sig_header) {
        Some(parts) => parts,
        None => return false,
    };

    let Ok(ts_int) = timestamp.parse::<i64>() else {
        return false;
    };
    if (now_ts - ts_int).abs() > STRIPE_TOLERANCE_SECS {
        return false;
    }

    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(value) => value,
        Err(_) => return false,
    };
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let computed = to_hex_lower(&mac.finalize().into_bytes());

    constant_time_eq(&computed, expected_sig)
}

fn parse_signature_header(sig_header: &str) -> Option<(&str, &str)> {
    let mut timestamp: Option<&str> = None;
    let mut signature: Option<&str> = None;

    for part in sig_header.split(',') {
        if let Some(value) = part.strip_prefix("t=") {
            timestamp = Some(value);
        } else if let Some(value) = part.strip_prefix("v1=") {
            signature = Some(value);
        }
    }

    match (timestamp, signature) {
        (Some(ts), Some(sig)) => Some((ts, sig)),
        _ => None,
    }
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn to_hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use uuid::Uuid;

    fn stripe_sig(secret: &str, ts: i64, payload: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac key");
        mac.update(ts.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        let sig = to_hex_lower(&mac.finalize().into_bytes());
        format!("t={ts},v1={sig}")
    }

    #[test]
    fn stripe_signature_valid_for_recent_timestamp() {
        let secret = Uuid::new_v4().to_string();
        let payload = br#"{"id":"evt_1","type":"checkout.session.completed"}"#;
        let now = 1_700_000_000;
        let header = stripe_sig(&secret, now, payload);
        assert!(verify_stripe_signature_at(
            &header,
            payload,
            &secret,
            now + 30
        ));
    }

    #[test]
    fn stripe_signature_rejects_replayed_timestamp() {
        let secret = Uuid::new_v4().to_string();
        let payload = br#"{"id":"evt_1","type":"checkout.session.completed"}"#;
        let ts = 1_700_000_000;
        let header = stripe_sig(&secret, ts, payload);
        assert!(!verify_stripe_signature_at(
            &header,
            payload,
            &secret,
            ts + STRIPE_TOLERANCE_SECS + 1
        ));
    }

    #[test]
    fn subscription_status_only_grants_for_active_or_trialing() {
        assert!(should_be_premium("active"));
        assert!(should_be_premium("trialing"));
        assert!(!should_be_premium("past_due"));
        assert!(!should_be_premium("unpaid"));
        assert!(!should_be_premium("canceled"));
    }
}
