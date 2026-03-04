/**
 * Premium (GoPro) — subscription status and activation.
 *
 * Routes (mounted under /api/v1/premium):
 *   GET    /        — current user's premium status + feature list
 *   POST   /activate — activate GoPro via promo code (early access / CS grants)
 *   DELETE /        — self-service cancel (sets is_premium = FALSE)
 *   POST   /webhook  — Stripe webhook (checkout.session.completed / subscription.deleted)
 *
 * Promo code activation:
 *   Set GOPRO_PROMO_CODES env var to a comma-separated list of codes (case-insensitive).
 *   e.g. "YAPPER2026,BETA100" — share these with early adopters until Stripe is wired up.
 *
 * Stripe webhook:
 *   When STRIPE_WEBHOOK_SECRET is set the endpoint verifies the Stripe-Signature header
 *   (HMAC-SHA256 of "<timestamp>.<raw_body>") before processing events.
 *   Events handled:
 *     checkout.session.completed    → set is_premium = TRUE
 *     customer.subscription.updated → set is_premium = TRUE (renewal)
 *     customer.subscription.deleted → set is_premium = FALSE
 *   Stripe checkout sessions must include metadata: { "user_id": "<uuid>" }
 */
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

// ─── Feature catalogue ────────────────────────────────────────────────────────

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

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_status))
        .route("/activate", post(activate))
        .route("/", delete(cancel))
        .route("/webhook", post(stripe_webhook))
}

// ─── GET /api/v1/premium ──────────────────────────────────────────────────────

async fn get_status(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query(
        "SELECT is_premium, premium_since FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_one(state.db.pool())
    .await?;

    let is_premium: bool = row.try_get("is_premium").unwrap_or(false);
    let premium_since: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("premium_since").ok().flatten();

    Ok(Json(serde_json::json!({
        "is_premium":     is_premium,
        "plan":           if is_premium { "gopro" } else { "free" },
        "premium_since":  premium_since.map(|t| t.to_rfc3339()),
        "features":       GOPRO_FEATURES,
    })))
}

// ─── POST /api/v1/premium/activate ───────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivateInput {
    promo_code: String,
}

async fn activate(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ActivateInput>,
) -> AppResult<impl IntoResponse> {
    // Validate promo code against GOPRO_PROMO_CODES env var (comma-separated, case-insensitive)
    let valid_codes: Vec<String> = std::env::var("GOPRO_PROMO_CODES")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    if valid_codes.is_empty() {
        return Err(AppError::BadRequest(
            "Promo code activation is not enabled on this server".into(),
        ));
    }

    if !valid_codes.contains(&body.promo_code.trim().to_uppercase()) {
        return Err(AppError::BadRequest("Invalid promo code".into()));
    }

    // Check current status
    let row = sqlx::query("SELECT is_premium FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(auth.user_id)
        .fetch_one(state.db.pool())
        .await?;

    let already: bool = row.try_get("is_premium").unwrap_or(false);
    if already {
        return Err(AppError::BadRequest("Your account is already GoPro".into()));
    }

    sqlx::query(
        "UPDATE users SET is_premium = TRUE, premium_since = NOW() WHERE id = $1",
    )
    .bind(auth.user_id)
    .execute(state.db.pool())
    .await?;

    tracing::info!(user_id = %auth.user_id, "GoPro activated via promo code");

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "is_premium":  true,
            "plan":        "gopro",
            "message":     "GoPro activated. Welcome to the hype!",
        })),
    ))
}

// ─── DELETE /api/v1/premium ───────────────────────────────────────────────────

async fn cancel(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    sqlx::query(
        "UPDATE users SET is_premium = FALSE, premium_since = NULL WHERE id = $1",
    )
    .bind(auth.user_id)
    .execute(state.db.pool())
    .await?;

    tracing::info!(user_id = %auth.user_id, "GoPro cancelled");
    Ok(StatusCode::NO_CONTENT)
}

// ─── POST /api/v1/premium/webhook ────────────────────────────────────────────

/// Stripe webhook receiver.
///
/// This route is intentionally excluded from the CSRF middleware because Stripe
/// sends requests from its own servers (no cookie). The Stripe-Signature HMAC
/// check replaces CSRF protection here.
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let secret = match std::env::var("STRIPE_WEBHOOK_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            tracing::warn!("STRIPE_WEBHOOK_SECRET not set — webhook endpoint is a no-op");
            return StatusCode::OK;
        }
    };

    let sig_header = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => return StatusCode::BAD_REQUEST,
    };

    if !verify_stripe_signature(&sig_header, &body, &secret) {
        tracing::warn!("Stripe webhook signature verification failed");
        return StatusCode::UNAUTHORIZED;
    }

    let event: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let event_type = event["type"].as_str().unwrap_or("");
    let obj = &event["data"]["object"];

    match event_type {
        "checkout.session.completed" | "customer.subscription.updated" => {
            if let Some(uid) = extract_user_id(obj) {
                let _ = sqlx::query(
                    "UPDATE users SET is_premium = TRUE, premium_since = NOW() WHERE id = $1",
                )
                .bind(uid)
                .execute(state.db.pool())
                .await;
                tracing::info!(user_id = %uid, event = event_type, "GoPro activated via Stripe");
            }
        }
        "customer.subscription.deleted" => {
            if let Some(uid) = extract_user_id(obj) {
                let _ = sqlx::query(
                    "UPDATE users SET is_premium = FALSE, premium_since = NULL WHERE id = $1",
                )
                .bind(uid)
                .execute(state.db.pool())
                .await;
                tracing::info!(user_id = %uid, "GoPro cancelled via Stripe");
            }
        }
        _ => {} // Unhandled event — acknowledge with 200 OK
    }

    StatusCode::OK
}

fn extract_user_id(obj: &serde_json::Value) -> Option<Uuid> {
    obj["metadata"]["user_id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
}

// ─── Stripe signature verification ───────────────────────────────────────────

/// Verifies Stripe webhook signature.
/// Header format: `t=<unix_ts>,v1=<hmac_hex>`
/// Signed payload: `<t>.<raw_body_bytes_as_string>`
fn verify_stripe_signature(sig_header: &str, body: &[u8], secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut timestamp: Option<&str> = None;
    let mut expected_sig: Option<&str> = None;

    for part in sig_header.split(',') {
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = Some(t);
        } else if let Some(s) = part.strip_prefix("v1=") {
            expected_sig = Some(s);
        }
    }

    let (Some(ts), Some(expected)) = (timestamp, expected_sig) else {
        return false;
    };

    // Stripe signed payload: "{timestamp}.{raw_body}"
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(ts.as_bytes());
    mac.update(b".");
    mac.update(body);

    let computed = mac
        .finalize()
        .into_bytes()
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write;
            write!(s, "{b:02x}").unwrap();
            s
        });

    // Constant-time comparison to prevent timing attacks
    if computed.len() != expected.len() {
        return false;
    }
    computed
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
