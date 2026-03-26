/**
 * Support â€” user-submitted tickets forwarded to HubSpot CRM.
 *
 * Routes (mounted under /api/v2/support):
 *   POST  /tickets          â€” create a ticket (DB insert + HubSpot ticket creation)
 *   GET   /tickets          â€” list the authenticated user's own tickets
 *   POST  /webhooks/hubspot â€” inbound webhook for HubSpot ticket status updates
 *
 * HubSpot integration:
 *   - Requires HUBSPOT_ACCESS_TOKEN env var (Private App token).
 *   - If the var is absent the ticket is still saved locally; a warning is logged.
 *   - The HubSpot ticket ID is stored in support_tickets.hubspot_ticket_id.
 *   - Pipeline: default (0), Stage: New (1).
 *   - Priority mapping: lowâ†’LOW, mediumâ†’MEDIUM, highâ†’HIGH, urgentâ†’URGENT.
 *   - Ticket type is prefixed in the subject: "[Bug]", "[Idea]", "[Improvement]".
 *   - Inbound webhooks verified via HMAC-SHA256 (HUBSPOT_CLIENT_SECRET).
 */
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, RateLimiter};
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::Row;
use std::num::NonZeroU32;
use uuid::Uuid;

use crate::{
    auth::AuthDevice,
    error::{AppError, AppResult},
    AppState,
};

// â”€â”€â”€ Constants â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const HUBSPOT_TICKETS_API: &str = "https://api.hubapi.com/crm/v3/objects/tickets";
const MAX_TICKETS_PER_USER: i64 = 50;

/// Rate limiter: 5 ticket creations per hour per user.
static TICKET_RATE_LIMITER: Lazy<RateLimiter<Uuid, DefaultKeyedStateStore<Uuid>, DefaultClock>> =
    Lazy::new(|| {
        RateLimiter::keyed(
            governor::Quota::with_period(std::time::Duration::from_secs(60 * 60))
                .expect("valid ticket quota")
                .allow_burst(NonZeroU32::new(5).expect("non-zero burst")),
        )
    });
const MAX_SUBJECT_LEN: usize = 200;
const MAX_DESCRIPTION_LEN: usize = 2000;
static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").expect("email regex")
});
static SECRET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(bearer\s+[A-Z0-9._-]+|api[_ -]?key\s*[:=]\s*\S+|password\s*[:=]\s*\S+)\b")
        .expect("secret regex")
});
static SSN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("ssn regex"));
static PAN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:\d[ -]*?){13,19}\b").expect("pan regex"));

// â”€â”€â”€ Router â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tickets", post(create_ticket))
        .route("/tickets", get(list_tickets))
        .route("/webhooks/hubspot", post(hubspot_webhook))
}

// â”€â”€â”€ Input / Output types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateTicketInput {
    /// "bug" | "idea" | "improvement"
    ticket_type: String,
    /// Short summary (1â€“200 chars)
    subject: String,
    /// Full description (1â€“2000 chars)
    description: String,
    /// "low" | "medium" | "high" | "urgent" â€” only meaningful for bugs
    #[serde(default = "default_priority")]
    priority: String,
}

fn default_priority() -> String {
    "medium".to_string()
}

// â”€â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn validate_ticket_type(t: &str) -> bool {
    matches!(t, "bug" | "idea" | "improvement")
}

fn validate_priority(p: &str) -> bool {
    matches!(p, "low" | "medium" | "high" | "urgent")
}

fn hubspot_priority(p: &str) -> &'static str {
    match p {
        "low" => "LOW",
        "high" => "HIGH",
        "urgent" => "URGENT",
        _ => "MEDIUM",
    }
}

fn ticket_type_prefix(t: &str) -> &'static str {
    match t {
        "bug" => "[Bug]",
        "idea" => "[Idea]",
        _ => "[Improvement]",
    }
}

fn include_hubspot_pii() -> bool {
    matches!(
        std::env::var("HUBSPOT_INCLUDE_PII")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1" | "true" | "yes")
    )
}

fn redact_support_text(input: &str) -> String {
    let redacted = EMAIL_RE.replace_all(input, "[redacted-email]");
    let redacted = SECRET_RE.replace_all(&redacted, "[redacted-secret]");
    let redacted = SSN_RE.replace_all(&redacted, "[redacted-ssn]");
    PAN_RE
        .replace_all(&redacted, "[redacted-card]")
        .into_owned()
}

fn build_hubspot_content(
    local_id: Uuid,
    user_id: Uuid,
    ticket_type: &str,
    description: &str,
    priority: &str,
    username: Option<&str>,
    user_email: Option<&str>,
) -> String {
    let mut content = redact_support_text(description);
    content.push_str("\n\n---\n");
    content.push_str(&format!(
        "Yapper ticket ID: {local_id}\nYapper user ID: {user_id}\nType: {ticket_type} | Priority: {priority}"
    ));

    if let (Some(username), Some(user_email)) = (username, user_email) {
        content.push_str(&format!("\nSubmitted by: @{username} ({user_email})"));
    }

    content
}

/// Validates CreateTicketInput fields; returns BadRequest on the first violation.
fn validate_input(body: &CreateTicketInput) -> AppResult<()> {
    if !validate_ticket_type(&body.ticket_type) {
        return Err(AppError::BadRequest(
            "ticket_type must be one of: bug, idea, improvement".into(),
        ));
    }
    if !validate_priority(&body.priority) {
        return Err(AppError::BadRequest(
            "priority must be one of: low, medium, high, urgent".into(),
        ));
    }
    if body.subject.is_empty() || body.subject.len() > MAX_SUBJECT_LEN {
        return Err(AppError::BadRequest(format!(
            "subject must be 1â€“{MAX_SUBJECT_LEN} characters"
        )));
    }
    if body.description.is_empty() || body.description.len() > MAX_DESCRIPTION_LEN {
        return Err(AppError::BadRequest(format!(
            "description must be 1â€“{MAX_DESCRIPTION_LEN} characters"
        )));
    }
    Ok(())
}

// â”€â”€â”€ POST /tickets â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Creates a support ticket, stores it locally, and forwards it to HubSpot.
async fn create_ticket(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(body): Json<CreateTicketInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_trusted()?;
    validate_input(&body)?;

    // Enforce hourly rate limit (5/hour/user)
    if TICKET_RATE_LIMITER.check_key(&auth.user_id).is_err() {
        return Err(AppError::RateLimited);
    }

    // Enforce per-user ticket cap
    let ticket_count: i64 = sqlx::query("SELECT COUNT(*) FROM support_tickets WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_one(state.db.pool())
        .await
        .map(|r| r.try_get::<i64, _>(0).unwrap_or(0))
        .unwrap_or(0);

    if ticket_count >= MAX_TICKETS_PER_USER {
        return Err(AppError::BadRequest(format!(
            "Maximum {MAX_TICKETS_PER_USER} support tickets per user"
        )));
    }

    let include_pii = include_hubspot_pii();
    let (user_email, username) = if include_pii {
        let user_row = sqlx::query("SELECT email, username FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_one(state.db.pool())
            .await?;
        (
            user_row.try_get::<String, _>("email").ok(),
            user_row.try_get::<String, _>("username").ok(),
        )
    } else {
        (None, None)
    };

    // Insert ticket locally first (HubSpot is best-effort)
    let ticket_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO support_tickets (id, user_id, ticket_type, subject, description, priority)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(ticket_id)
    .bind(auth.user_id)
    .bind(&body.ticket_type)
    .bind(&body.subject)
    .bind(&body.description)
    .bind(&body.priority)
    .execute(state.db.pool())
    .await?;

    // Forward to HubSpot (best-effort â€” log and continue if it fails)
    let hubspot_id = forward_to_hubspot(HubspotTicketForward {
        local_id: ticket_id,
        ticket_type: &body.ticket_type,
        subject: &body.subject,
        description: &body.description,
        priority: &body.priority,
        user_id: auth.user_id,
        user_email: user_email.as_deref(),
        username: username.as_deref(),
    })
    .await;

    match hubspot_id {
        Ok(ref hs_id) => {
            // Store the HubSpot ticket ID for reference
            let _ = sqlx::query(
                "UPDATE support_tickets SET hubspot_ticket_id = $1 WHERE id = $2",
            )
            .bind(hs_id)
            .bind(ticket_id)
            .execute(state.db.pool())
            .await
            .map_err(|e| {
                tracing::error!(ticket_id = %ticket_id, "Failed to store HubSpot ticket ID: {e}");
            });

            tracing::info!(
                user_id = %auth.user_id,
                ticket_id = %ticket_id,
                hubspot_ticket_id = %hs_id,
                ticket_type = %body.ticket_type,
                "Support ticket created and forwarded to HubSpot"
            );
        }
        Err(ref e) => {
            tracing::warn!(
                user_id = %auth.user_id,
                ticket_id = %ticket_id,
                "HubSpot forwarding failed (ticket still saved locally): {e}"
            );
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id":         ticket_id,
            "ticket_type": body.ticket_type,
            "subject":    body.subject,
            "priority":   body.priority,
            "status":     "open",
            "hubspot_id": hubspot_id.ok(),
        })),
    ))
}

// â”€â”€â”€ GET /tickets â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Returns the authenticated user's own submitted tickets (newest first, max 50).
async fn list_tickets(
    auth: AuthDevice,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    auth.require_trusted()?;
    let rows = sqlx::query(
        "SELECT id, ticket_type, subject, priority, status, hubspot_ticket_id, created_at
         FROM support_tickets
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT 50",
    )
    .bind(auth.user_id)
    .fetch_all(state.db.pool())
    .await?;

    let tickets: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":          r.try_get::<Uuid, _>("id").ok(),
                "ticket_type": r.try_get::<String, _>("ticket_type").unwrap_or_default(),
                "subject":     r.try_get::<String, _>("subject").unwrap_or_default(),
                "priority":    r.try_get::<String, _>("priority").unwrap_or_default(),
                "status":      r.try_get::<String, _>("status").unwrap_or_default(),
                "hubspot_id":  r.try_get::<Option<String>, _>("hubspot_ticket_id").ok().flatten(),
                "created_at":  r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                                 .ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "tickets": tickets })))
}

// â”€â”€â”€ HubSpot forwarding â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Sends the ticket to HubSpot CRM and returns the created HubSpot ticket ID.
/// Returns an error string if HUBSPOT_ACCESS_TOKEN is not set or the API call fails.
struct HubspotTicketForward<'a> {
    local_id: Uuid,
    ticket_type: &'a str,
    subject: &'a str,
    description: &'a str,
    priority: &'a str,
    user_id: Uuid,
    user_email: Option<&'a str>,
    username: Option<&'a str>,
}

async fn forward_to_hubspot(ticket: HubspotTicketForward<'_>) -> Result<String, String> {
    let access_token = std::env::var("HUBSPOT_ACCESS_TOKEN")
        .map_err(|_| "HUBSPOT_ACCESS_TOKEN not configured".to_string())?;

    let hs_subject = format!(
        "{} {}",
        ticket_type_prefix(ticket.ticket_type),
        ticket.subject
    );
    let hs_priority = hubspot_priority(ticket.priority);

    let content = build_hubspot_content(
        ticket.local_id,
        ticket.user_id,
        ticket.ticket_type,
        ticket.description,
        ticket.priority,
        ticket.username,
        ticket.user_email,
    );

    let body = serde_json::json!({
        "properties": {
            "hs_ticket_subject":  hs_subject,
            "content":            content,
            "hs_ticket_priority": hs_priority,
            "hs_pipeline":        "0",
            "hs_pipeline_stage":  "1",
            "source_type":        "EMAIL"
        }
    });

    static HTTP: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);
    let response = HTTP
        .post(HUBSPOT_TICKETS_API)
        .bearer_auth(&access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HubSpot request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("HubSpot returned {status}: {text}"));
    }

    let resp_body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("HubSpot response parse failed: {e}"))?;

    resp_body["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "HubSpot response missing id field".to_string())
}

// â”€â”€â”€ HubSpot inbound webhook â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Maximum age of a HubSpot webhook timestamp before we reject it (5 minutes).
const HUBSPOT_TIMESTAMP_MAX_AGE_SECS: i64 = 300;

/// Verifies the HubSpot v3 webhook signature.
/// Signature = Base64(HMAC-SHA256(client_secret, method + url + body + timestamp))
fn verify_hubspot_signature(
    client_secret: &str,
    method: &str,
    url: &str,
    body: &[u8],
    timestamp: &str,
    signature: &str,
) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(client_secret.as_bytes()) else {
        return false;
    };
    mac.update(method.as_bytes());
    mac.update(url.as_bytes());
    mac.update(body);
    mac.update(timestamp.as_bytes());

    // Decode the provided signature and use HMAC's constant-time verify_slice
    // instead of string comparison, which is vulnerable to timing attacks.
    let Ok(sig_bytes) = BASE64.decode(signature) else {
        return false;
    };
    mac.verify_slice(&sig_bytes).is_ok()
}

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

fn map_hubspot_stage_to_status(stage: &str) -> Option<&'static str> {
    match stage {
        "1" => Some("open"),
        "2" => Some("in_progress"),
        "3" => Some("waiting"),
        "4" => Some("closed"),
        _ => None,
    }
}

/// Receives HubSpot webhook events and updates local ticket status.
async fn hubspot_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let client_secret = std::env::var("HUBSPOT_CLIENT_SECRET").map_err(|_| {
        tracing::warn!("HUBSPOT_CLIENT_SECRET not configured â€” rejecting webhook");
        AppError::Forbidden
    })?;

    let signature = headers
        .get("X-HubSpot-Signature-v3")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::BadRequest("Missing signature header".into()))?;

    let timestamp = headers
        .get("X-HubSpot-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::BadRequest("Missing timestamp header".into()))?;

    // Reject stale timestamps (>5 minutes old)
    if let Ok(ts_millis) = timestamp.parse::<i64>() {
        let now_millis = chrono::Utc::now().timestamp_millis();
        if (now_millis - ts_millis).abs() > HUBSPOT_TIMESTAMP_MAX_AGE_SECS * 1000 {
            return Err(AppError::BadRequest("Stale webhook timestamp".into()));
        }
    }

    // Build the URL HubSpot used when signing (we use a fixed path since
    // the full URL isn't available from headers alone; configure the webhook
    // target URL to match HUBSPOT_WEBHOOK_URL env var or fallback).
    let webhook_url = std::env::var("HUBSPOT_WEBHOOK_URL").unwrap_or_default();

    if !verify_hubspot_signature(
        &client_secret,
        "POST",
        &webhook_url,
        &body,
        timestamp,
        signature,
    ) {
        tracing::warn!("HubSpot webhook signature verification failed");
        return Err(AppError::Forbidden);
    }

    let events: Vec<serde_json::Value> =
        serde_json::from_slice(&body).map_err(|_| AppError::BadRequest("Invalid JSON".into()))?;

    for event in &events {
        process_hubspot_event(event, &state).await;
    }

    Ok(StatusCode::OK)
}

async fn process_hubspot_event(event: &serde_json::Value, state: &AppState) {
    let object_type = event["objectType"].as_str().unwrap_or("");
    if object_type != "TICKET" {
        return;
    }

    let property_name = event["propertyName"].as_str().unwrap_or("");
    if property_name != "hs_pipeline_stage" {
        return;
    }

    let hubspot_id = event["objectId"].as_i64().map(|id| id.to_string());
    let new_stage = event["propertyValue"].as_str().unwrap_or("");

    let Some(hubspot_id) = hubspot_id else {
        return;
    };
    let Some(new_status) = map_hubspot_stage_to_status(new_stage) else {
        tracing::debug!(stage = new_stage, "Unknown HubSpot pipeline stage");
        return;
    };

    let result = sqlx::query(
        "UPDATE support_tickets SET status = $1, updated_at = NOW() \
         WHERE hubspot_ticket_id = $2",
    )
    .bind(new_status)
    .bind(&hubspot_id)
    .execute(state.db.pool())
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::info!(
                hubspot_id = hubspot_id,
                new_status = new_status,
                "Updated support ticket status from HubSpot webhook"
            );
        }
        Ok(_) => {
            tracing::debug!(
                hubspot_id = hubspot_id,
                "HubSpot webhook for unknown ticket (not in our DB)"
            );
        }
        Err(e) => {
            tracing::error!(
                hubspot_id = hubspot_id,
                "Failed to update ticket status: {e}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_support_text_scrubs_common_pii_and_secrets() {
        let input = "Email alice@example.com SSN 123-45-6789 api_key=sekret 4111 1111 1111 1111";
        let redacted = redact_support_text(input);

        assert!(!redacted.contains("alice@example.com"));
        assert!(!redacted.contains("123-45-6789"));
        assert!(!redacted.contains("sekret"));
        assert!(!redacted.contains("4111 1111 1111 1111"));
        assert!(redacted.contains("[redacted-email]"));
        assert!(redacted.contains("[redacted-ssn]"));
        assert!(redacted.contains("[redacted-secret]"));
        assert!(redacted.contains("[redacted-card]"));
    }

    #[test]
    fn verify_hubspot_signature_accepts_valid_and_rejects_invalid() {
        let secret = "test-secret";
        let method = "POST";
        let url = "https://example.com/webhooks/hubspot";
        let body = b"{\"test\": true}";
        let timestamp = "1234567890";

        // Compute the correct signature
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(method.as_bytes());
        mac.update(url.as_bytes());
        mac.update(body);
        mac.update(timestamp.as_bytes());
        let valid_sig = BASE64.encode(mac.finalize().into_bytes());

        assert!(verify_hubspot_signature(
            secret, method, url, body, timestamp, &valid_sig
        ));
        assert!(!verify_hubspot_signature(
            secret, method, url, body, timestamp, "bad-sig"
        ));
        assert!(!verify_hubspot_signature(
            "wrong-secret",
            method,
            url,
            body,
            timestamp,
            &valid_sig
        ));
    }

    #[test]
    fn map_hubspot_stage_maps_known_stages() {
        assert_eq!(map_hubspot_stage_to_status("1"), Some("open"));
        assert_eq!(map_hubspot_stage_to_status("2"), Some("in_progress"));
        assert_eq!(map_hubspot_stage_to_status("3"), Some("waiting"));
        assert_eq!(map_hubspot_stage_to_status("4"), Some("closed"));
        assert_eq!(map_hubspot_stage_to_status("999"), None);
    }

    #[test]
    fn build_hubspot_content_excludes_direct_identifiers_by_default() {
        let content = build_hubspot_content(
            Uuid::nil(),
            Uuid::nil(),
            "bug",
            "Email alice@example.com password=sekret",
            "high",
            None,
            None,
        );

        assert!(!content.contains("alice@example.com"));
        assert!(!content.contains("sekret"));
        assert!(!content.contains("Submitted by:"));
        assert!(content.contains("Yapper user ID"));
    }
}
