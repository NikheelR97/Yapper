//! CSRF double-submit cookie middleware.
//!
//! Applied to the entire /api/v1 router, exempting /auth/* routes (those
//! endpoints receive no JWT and set the CSRF cookie themselves on success).
//!
//! For all other state-changing requests (POST/PUT/PATCH/DELETE), the client
//! must read the non-HttpOnly `csrf_token` cookie and echo it back as the
//! `X-CSRF-Token` request header. If they do not match, the request is rejected.

use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn csrf_check(req: Request, next: Next) -> Result<Response, StatusCode> {
    let method = req.method().clone();

    if matches!(
        method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return Ok(next.run(req).await);
    }

    // Axum strips the /api/v1 prefix before this middleware sees the path.
    const CSRF_EXEMPT: &[&str] = &[
        "/auth/login",
        "/auth/register",
        "/auth/verify-email",
        "/auth/forgot-password",
        "/auth/reset-password",
        "/auth/refresh",
        "/auth/logout",
        // Stripe webhooks are signed server-to-server callbacks (no CSRF cookie).
        "/premium/webhook",
    ];

    let path = req.uri().path();
    if CSRF_EXEMPT.contains(&path) {
        return Ok(next.run(req).await);
    }

    let headers = req.headers();

    let csrf_cookie: Option<String> = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            raw.split(';')
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
/// Not HttpOnly: JS must read the token to send X-CSRF-Token.
pub fn csrf_cookie_header(token: &str) -> String {
    let secure_flag = if should_use_secure_cookie() {
        "; Secure"
    } else {
        ""
    };
    format!("csrf_token={token}; SameSite=Strict; Path=/; Max-Age=86400{secure_flag}")
}

/// Clear CSRF cookie on logout.
pub fn clear_csrf_cookie() -> String {
    let secure_flag = if should_use_secure_cookie() {
        "; Secure"
    } else {
        ""
    };
    format!("csrf_token=; SameSite=Strict; Path=/; Max-Age=0{secure_flag}")
}

fn should_use_secure_cookie() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or_else(|_| std::env::var("FLY_APP_NAME").is_ok())
}
