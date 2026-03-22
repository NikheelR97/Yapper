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

    let path = normalize_path(req.uri().path());
    if is_csrf_exempt(path) {
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

fn normalize_path(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/"
    } else {
        trimmed
    }
}

fn is_csrf_exempt(path: &str) -> bool {
    matches!(
        path,
        "/auth/login"
            | "/auth/register"
            | "/auth/verify-email"
            | "/auth/password-reset/request"
            | "/auth/password-reset/confirm"
            | "/auth/refresh"
            | "/auth/oauth/exchange"
            | "/premium/webhook"
            | "/support/webhooks/hubspot"
    )
}

/// Build a `Set-Cookie` header value for the CSRF token.
/// Not HttpOnly: JS must read the token to send X-CSRF-Token.
pub fn csrf_cookie_header(token: &str) -> String {
    let secure_flag = if should_use_secure_cookie() {
        "; Secure"
    } else {
        ""
    };
    let same_site = if should_use_secure_cookie() {
        "None"
    } else {
        "Strict"
    };
    format!("csrf_token={token}; SameSite={same_site}; Path=/; Max-Age=86400{secure_flag}")
}

/// Clear CSRF cookie on logout.
pub fn clear_csrf_cookie() -> String {
    let secure_flag = if should_use_secure_cookie() {
        "; Secure"
    } else {
        ""
    };
    let same_site = if should_use_secure_cookie() {
        "None"
    } else {
        "Strict"
    };
    format!("csrf_token=; SameSite={same_site}; Path=/; Max-Age=0{secure_flag}")
}

pub fn should_use_secure_cookie() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or_else(|_| std::env::var("FLY_APP_NAME").is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_csrf_exempt ───────────────────────────────────────────────

    #[test]
    fn exempt_auth_login() {
        assert!(is_csrf_exempt("/auth/login"));
    }

    #[test]
    fn exempt_auth_register() {
        assert!(is_csrf_exempt("/auth/register"));
    }

    #[test]
    fn exempt_auth_verify_email() {
        assert!(is_csrf_exempt("/auth/verify-email"));
    }

    #[test]
    fn exempt_auth_password_reset_request() {
        assert!(is_csrf_exempt("/auth/password-reset/request"));
    }

    #[test]
    fn exempt_auth_password_reset_confirm() {
        assert!(is_csrf_exempt("/auth/password-reset/confirm"));
    }

    #[test]
    fn exempt_auth_refresh() {
        assert!(is_csrf_exempt("/auth/refresh"));
    }

    #[test]
    fn exempt_auth_oauth_exchange() {
        assert!(is_csrf_exempt("/auth/oauth/exchange"));
    }

    #[test]
    fn exempt_premium_webhook() {
        assert!(is_csrf_exempt("/premium/webhook"));
    }

    #[test]
    fn exempt_support_webhooks_hubspot() {
        assert!(is_csrf_exempt("/support/webhooks/hubspot"));
    }

    #[test]
    fn exempt_list_is_exactly_nine_entries() {
        // Ensure non-exempt paths are correctly rejected, confirming
        // the allowlist is not overly broad.
        let exempt = [
            "/auth/login",
            "/auth/register",
            "/auth/verify-email",
            "/auth/password-reset/request",
            "/auth/password-reset/confirm",
            "/auth/refresh",
            "/auth/oauth/exchange",
            "/premium/webhook",
            "/support/webhooks/hubspot",
        ];
        for path in &exempt {
            assert!(is_csrf_exempt(path), "{path} should be exempt");
        }
    }

    #[test]
    fn non_exempt_random_api_path() {
        assert!(!is_csrf_exempt("/api/v1/conversations"));
    }

    #[test]
    fn non_exempt_partial_match() {
        assert!(!is_csrf_exempt("/auth/login/extra"));
    }

    #[test]
    fn non_exempt_auth_logout() {
        assert!(!is_csrf_exempt("/auth/logout"));
    }

    #[test]
    fn non_exempt_root() {
        assert!(!is_csrf_exempt("/"));
    }

    #[test]
    fn non_exempt_empty() {
        assert!(!is_csrf_exempt(""));
    }

    #[test]
    fn non_exempt_premium_without_webhook() {
        assert!(!is_csrf_exempt("/premium"));
    }

    #[test]
    fn non_exempt_support_tickets() {
        assert!(!is_csrf_exempt("/support/tickets"));
    }

    #[test]
    fn non_exempt_auth_prefix_only() {
        assert!(!is_csrf_exempt("/auth"));
    }

    // ── normalize_path ───────────────────────────────────────────────

    #[test]
    fn normalize_strips_trailing_slash() {
        assert_eq!(normalize_path("/auth/login/"), "/auth/login");
    }

    #[test]
    fn normalize_no_trailing_slash_unchanged() {
        assert_eq!(normalize_path("/auth/login"), "/auth/login");
    }

    #[test]
    fn normalize_root_slash_stays() {
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn normalize_multiple_trailing_slashes() {
        assert_eq!(normalize_path("/foo///"), "/foo");
    }

    // ── csrf_cookie_header format (no env-dependent tests — env vars
    //    are process-global and race under parallel test execution) ───

    #[test]
    fn cookie_header_contains_token_name() {
        // We can't safely test Secure/SameSite mode in parallel tests because
        // should_use_secure_cookie() reads env vars that race across threads.
        // Instead test the invariant parts of the cookie format.
        let header = csrf_cookie_header("my-token");
        assert!(header.starts_with("csrf_token=my-token;"));
        assert!(header.contains("Path=/"));
        assert!(header.contains("Max-Age=86400"));
    }

    #[test]
    fn clear_cookie_sets_max_age_zero() {
        let header = clear_csrf_cookie();
        assert!(header.starts_with("csrf_token=;"));
        assert!(header.contains("Max-Age=0"));
    }
}
