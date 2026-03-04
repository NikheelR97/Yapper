use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

use super::service;

#[derive(Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ActivateInput {
    #[validate(length(min = 4, max = 64))]
    promo_code: String,
}

pub async fn get_status(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let status = service::load_status(auth.user_id, &state).await?;
    Ok(Json(status_payload(&status, None)))
}

pub async fn activate(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ActivateInput>,
) -> AppResult<impl IntoResponse> {
    body.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    service::check_promo_rate_limit(auth.user_id)?;
    let normalized_code = service::normalize_promo_code(&body.promo_code)?;
    let status = service::activate_with_promo(auth.user_id, &normalized_code, &state).await?;

    tracing::info!(user_id = %auth.user_id, "GoPro activated via promo code");
    Ok((
        StatusCode::OK,
        Json(status_payload(&status, Some("GoPro activated"))),
    ))
}

pub async fn cancel(auth: AuthUser, State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let status = service::disable_local_access(auth.user_id, &state).await?;
    tracing::info!(
        user_id = %auth.user_id,
        "GoPro access disabled by user (local entitlement only)"
    );

    Ok((
        StatusCode::OK,
        Json(status_payload(
            &status,
            Some("GoPro access disabled on this account"),
        )),
    ))
}

/// Stripe webhook receiver.
///
/// This route is intentionally excluded from CSRF middleware because Stripe
/// sends requests from Stripe servers (no browser cookie). Stripe signature
/// validation replaces CSRF checks for this endpoint.
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let secret = match std::env::var("STRIPE_WEBHOOK_SECRET") {
        Ok(value) if !value.is_empty() => value,
        _ => {
            tracing::warn!("STRIPE_WEBHOOK_SECRET not set; webhook endpoint is disabled");
            return StatusCode::OK;
        }
    };

    service::process_stripe_webhook(&headers, &body, &secret, &state).await
}

fn status_payload(status: &service::PremiumStatus, message: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "is_premium": status.is_premium,
        "plan": status.plan(),
        "premium_since": status.premium_since.map(|ts| ts.to_rfc3339()),
        "features": service::gopro_features(),
        "message": message,
    })
}
