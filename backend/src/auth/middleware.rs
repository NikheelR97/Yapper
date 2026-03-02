use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::{net::IpAddr, time::Instant};

use super::service::validate_access_token;
use crate::{error::AppError, AppState};

// ─── JWT Extractor ────────────────────────────────────────────────────────────

/// Authenticated user context extracted from the `Authorization: Bearer` header.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    #[allow(dead_code)]
    pub account_type: String,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized.into_response())?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized.into_response())?;

        let claims =
            validate_access_token(token, &state.jwt_keys).map_err(|e| e.into_response())?;

        Ok(AuthUser {
            user_id: claims.claims.sub,
            account_type: claims.claims.account_type,
        })
    }
}

// ─── Optional auth extractor ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

#[axum::async_trait]
impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(header) = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
        else {
            return Ok(OptionalAuthUser(None));
        };

        let Some(token) = header.strip_prefix("Bearer ") else {
            return Ok(OptionalAuthUser(None));
        };

        let Ok(claims) = validate_access_token(token, &state.jwt_keys) else {
            return Ok(OptionalAuthUser(None));
        };

        Ok(OptionalAuthUser(Some(AuthUser {
            user_id: claims.claims.sub,
            account_type: claims.claims.account_type,
        })))
    }
}

// ─── Role guard helpers ───────────────────────────────────────────────────────

#[allow(dead_code)]
impl AuthUser {
    pub fn require_parent(&self) -> Result<(), AppError> {
        if self.account_type == "parent" {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn is_child(&self) -> bool {
        self.account_type == "child"
    }

    pub fn is_bot(&self) -> bool {
        self.account_type == "bot"
    }
}

// ─── Login rate limiter ───────────────────────────────────────────────────────

/// Per-IP failed login tracker.
/// 5 failed attempts → 15-minute lockout. No Redis needed.
pub struct LoginRateLimiter {
    state: DashMap<IpAddr, (u32, Option<Instant>)>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self {
            state: DashMap::new(),
        }
    }

    pub fn is_locked(&self, ip: IpAddr) -> bool {
        if let Some(entry) = self.state.get(&ip) {
            let (_, lockout_until) = *entry;
            if let Some(until) = lockout_until {
                if Instant::now() < until {
                    return true;
                }
            }
        }
        false
    }

    /// Record a failed login attempt. Returns true if IP is now locked out.
    pub fn record_failure(&self, ip: IpAddr) -> bool {
        let mut entry = self.state.entry(ip).or_insert((0, None));
        let (count, lockout) = &mut *entry;

        // Clear expired lockout
        if let Some(until) = lockout {
            if Instant::now() >= *until {
                *count = 0;
                *lockout = None;
            }
        }

        *count += 1;
        if *count >= 5 {
            *lockout = Some(Instant::now() + std::time::Duration::from_secs(900));
            return true;
        }
        false
    }

    pub fn record_success(&self, ip: IpAddr) {
        self.state.remove(&ip);
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_login_rate_limiter_locks_after_5_failures() {
        let limiter = LoginRateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));

        for i in 0..4 {
            assert!(
                !limiter.record_failure(ip),
                "Should not lock on iteration {i}"
            );
            assert!(!limiter.is_locked(ip));
        }
        assert!(limiter.record_failure(ip), "5th failure should lock");
        assert!(limiter.is_locked(ip));
    }

    #[test]
    fn test_login_rate_limiter_clears_on_success() {
        let limiter = LoginRateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 5));

        limiter.record_failure(ip);
        limiter.record_failure(ip);
        limiter.record_success(ip);

        for _ in 0..4 {
            assert!(!limiter.record_failure(ip));
        }
        assert!(limiter.record_failure(ip));
    }
}
