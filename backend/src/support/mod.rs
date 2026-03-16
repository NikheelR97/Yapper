/**
 * Support — user-submitted tickets forwarded to HubSpot CRM.
 *
 * Routes (mounted under /api/v1/support):
 *   POST  /tickets     — create a ticket (DB insert + HubSpot ticket creation)
 *   GET   /tickets     — list the authenticated user's own tickets
 *
 * HubSpot integration:
 *   - Requires HUBSPOT_ACCESS_TOKEN env var (Private App token).
 *   - If the var is absent the ticket is still saved locally; a warning is logged.
 *   - The HubSpot ticket ID is stored in support_tickets.hubspot_ticket_id.
 *   - Pipeline: default (0), Stage: New (1).
 *   - Priority mapping: low→LOW, medium→MEDIUM, high→HIGH, urgent→URGENT.
 *   - Ticket type is prefixed in the subject: "[Bug]", "[Idea]", "[Improvement]".
 */
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthDevice,
    error::{AppError, AppResult},
    AppState,
};

// ─── Constants ────────────────────────────────────────────────────────────────

const HUBSPOT_TICKETS_API: &str = "https://api.hubapi.com/crm/v3/objects/tickets";
const MAX_TICKETS_PER_USER: i64 = 50;
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

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tickets", post(create_ticket))
        .route("/tickets", get(list_tickets))
}

// ─── Input / Output types ─────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateTicketInput {
    /// "bug" | "idea" | "improvement"
    ticket_type: String,
    /// Short summary (1–200 chars)
    subject: String,
    /// Full description (1–2000 chars)
    description: String,
    /// "low" | "medium" | "high" | "urgent" — only meaningful for bugs
    #[serde(default = "default_priority")]
    priority: String,
}

fn default_priority() -> String {
    "medium".to_string()
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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
            "subject must be 1–{MAX_SUBJECT_LEN} characters"
        )));
    }
    if body.description.is_empty() || body.description.len() > MAX_DESCRIPTION_LEN {
        return Err(AppError::BadRequest(format!(
            "description must be 1–{MAX_DESCRIPTION_LEN} characters"
        )));
    }
    Ok(())
}

// ─── POST /tickets ─────────────────────────────────────────────────────────────

/// Creates a support ticket, stores it locally, and forwards it to HubSpot.
async fn create_ticket(
    auth: AuthDevice,
    State(state): State<AppState>,
    Json(body): Json<CreateTicketInput>,
) -> AppResult<impl IntoResponse> {
    auth.require_trusted()?;
    validate_input(&body)?;

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

    // Forward to HubSpot (best-effort — log and continue if it fails)
    let hubspot_id = forward_to_hubspot(
        ticket_id,
        &body.ticket_type,
        &body.subject,
        &body.description,
        &body.priority,
        auth.user_id,
        user_email.as_deref(),
        username.as_deref(),
    )
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

// ─── GET /tickets ──────────────────────────────────────────────────────────────

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

// ─── HubSpot forwarding ───────────────────────────────────────────────────────

/// Sends the ticket to HubSpot CRM and returns the created HubSpot ticket ID.
/// Returns an error string if HUBSPOT_ACCESS_TOKEN is not set or the API call fails.
async fn forward_to_hubspot(
    local_id: Uuid,
    ticket_type: &str,
    subject: &str,
    description: &str,
    priority: &str,
    user_id: Uuid,
    user_email: Option<&str>,
    username: Option<&str>,
) -> Result<String, String> {
    let access_token = std::env::var("HUBSPOT_ACCESS_TOKEN")
        .map_err(|_| "HUBSPOT_ACCESS_TOKEN not configured".to_string())?;

    let hs_subject = format!("{} {}", ticket_type_prefix(ticket_type), subject);
    let hs_priority = hubspot_priority(priority);

    let content = build_hubspot_content(
        local_id,
        user_id,
        ticket_type,
        description,
        priority,
        username,
        user_email,
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

    let client = reqwest::Client::new();
    let response = client
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
