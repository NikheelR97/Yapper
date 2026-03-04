//! CSRF double-submit cookie middleware.
//!
//! Applied to the entire /api/v1 router, exempting /auth/* routes (those
//! endpoints receive no JWT and set the CSRF cookie themselves on success).
//!
//! For all other state-changing requests (POST/PUT/PATCH/DELETE), the client
//! must read the non-HttpOnly `csrf_token` cookie and echo it back as the
//! `X-CSRF-Token` request header. If they don't match → 403.
//!
//! Defense-in-depth alongside SameSite=Strict cookies and strict CORS.

use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn csrf_check(req: Request, next: Next) -> Result<Response, StatusCode> {
    let method = req.method().clone();

    // Safe methods never change state — skip.
    if matches!(
        method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return Ok(next.run(req).await);
    }

    // Exempt specific auth routes — they issue the CSRF cookie and cannot require one they haven't
    // set yet. Explicit allowlist prevents accidentally exempting future routes under /auth/.
    // NOTE: Axum strips the /api/v1 prefix before this middleware sees the path, so the path
    // here is e.g. "/auth/refresh", not "/api/v1/auth/refresh".
    // OAuth routes (/auth/oauth/*) are mounted at the top level outside api_router() so this
    // middleware never runs for them — no need to list them here.
    const CSRF_EXEMPT: &[&str] = &[
        "/auth/login",
        "/auth/register",
        "/auth/verify-email",
        "/auth/forgot-password",
        "/auth/reset-password",
        "/auth/refresh",
        "/auth/logout",
        // Stripe webhook arrives from Stripe's servers (no cookie/CSRF token).
        // The Stripe-Signature HMAC check in the handler replaces CSRF protection.
        "/premium/webhook",
    ];
    let path = req.uri().path();
    if CSRF_EXEMPT.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(req).await);
    }

    let headers = req.headers();

    let csrf_cookie: Option<String> = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(str::trim)
                .find(|p| p.starts_with("csrf_token="))
                .map(|p| p.trim_start_matches("csrf_token=").to_string())
        });

    let csrf_header: Option<String> = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match (csrf_cookie, csrf_header) {
        (Some(cookie), Some(header)) if !cookie.is_empty() && cookie == header => {
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::FORBIDDEN),
    }
}

/// Build a `Set-Cookie` header value for the CSRF token.
/// NOT HttpOnly — JS must be able to read it to include in the X-CSRF-Token header.
pub fn csrf_cookie_header(token: &str) -> String {
    // No Secure flag — this cookie is intentionally readable by JS (not HttpOnly) and
    // carries no sensitive data; SameSite=Strict provides the CSRF protection.
    // Omitting Secure allows it to be stored in HTTP-only dev environments (Tauri/Capacitor).
    format!("csrf_token={token}; SameSite=Strict; Path=/; Max-Age=86400")
}

/// Clears the CSRF cookie on logout.
pub fn clear_csrf_cookie() -> String {
    "csrf_token=; SameSite=Strict; Path=/; Max-Age=0".to_string()
}
