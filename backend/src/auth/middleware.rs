use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::{
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

use super::service::validate_access_token;
use crate::{
    devices::{self, DeviceTrustState},
    error::AppError,
    AppState,
};

// ─── JWT Extractor ────────────────────────────────────────────────────────────

/// Authenticated user context extracted from the `Authorization: Bearer` header.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    #[allow(dead_code)]
    pub device_id: Option<uuid::Uuid>,
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
            device_id: claims.claims.device_id,
            account_type: claims.claims.account_type,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthDevice {
    pub user_id: uuid::Uuid,
    pub device_id: uuid::Uuid,
    pub signal_device_id: i32,
    pub trust_state: DeviceTrustState,
    #[allow(dead_code)]
    pub account_type: String,
}

impl AuthDevice {
    pub fn require_trusted(&self) -> Result<(), AppError> {
        if self.trust_state == DeviceTrustState::Trusted {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthDevice {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        let device_id = auth
            .device_id
            .ok_or_else(|| AppError::Unauthorized.into_response())?;

        let device = devices::get_device_for_user(auth.user_id, device_id, state)
            .await
            .map_err(|e| e.into_response())?;

        if device.revoked_at.is_some() || device.trust_state == DeviceTrustState::Revoked {
            return Err(AppError::Unauthorized.into_response());
        }

        Ok(Self {
            user_id: auth.user_id,
            device_id,
            signal_device_id: device.signal_device_id,
            trust_state: device.trust_state,
            account_type: auth.account_type,
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
            device_id: claims.claims.device_id,
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
    state: DashMap<IpAddr, LoginRateLimitEntry>,
    last_gc: Mutex<Instant>,
}

#[derive(Clone, Copy)]
struct LoginRateLimitEntry {
    failures: u32,
    lockout_until: Option<Instant>,
    last_event: Instant,
}

impl LoginRateLimiter {
    const LOCKOUT_SECS: u64 = 900;
    const GC_INTERVAL_SECS: u64 = 300;
    const MAX_ENTRIES: usize = 50_000;

    pub fn new() -> Self {
        Self {
            state: DashMap::new(),
            last_gc: Mutex::new(Instant::now()),
        }
    }

    pub fn is_locked(&self, ip: IpAddr) -> bool {
        self.maybe_gc();
        if let Some(entry) = self.state.get(&ip) {
            if let Some(until) = entry.lockout_until {
                if Instant::now() < until {
                    return true;
                }
            }
        }
        false
    }

    /// Record a failed login attempt. Returns true if IP is now locked out.
    pub fn record_failure(&self, ip: IpAddr) -> bool {
        self.maybe_gc();
        self.enforce_capacity();

        let now = Instant::now();
        let mut entry = self.state.entry(ip).or_insert(LoginRateLimitEntry {
            failures: 0,
            lockout_until: None,
            last_event: now,
        });

        // Clear expired lockout
        if let Some(until) = entry.lockout_until {
            if now >= until {
                entry.failures = 0;
                entry.lockout_until = None;
            }
        }

        entry.failures += 1;
        entry.last_event = now;
        if entry.failures >= 5 {
            entry.lockout_until = Some(now + Duration::from_secs(Self::LOCKOUT_SECS));
            return true;
        }
        false
    }

    pub fn record_success(&self, ip: IpAddr) {
        self.maybe_gc();
        self.state.remove(&ip);
    }

    fn maybe_gc(&self) {
        let now = Instant::now();
        let mut last_gc = self
            .last_gc
            .lock()
            .expect("login limiter gc mutex poisoned");
        if now.duration_since(*last_gc) < Duration::from_secs(Self::GC_INTERVAL_SECS) {
            return;
        }
        *last_gc = now;
        drop(last_gc);

        let mut expired = Vec::new();
        for entry in self.state.iter() {
            let remove = match entry.lockout_until {
                Some(until) => now >= until,
                None => {
                    now.duration_since(entry.last_event) >= Duration::from_secs(Self::LOCKOUT_SECS)
                }
            };
            if remove {
                expired.push(*entry.key());
            }
        }
        for ip in expired {
            self.state.remove(&ip);
        }
    }

    fn enforce_capacity(&self) {
        if self.state.len() < Self::MAX_ENTRIES {
            return;
        }

        let now = Instant::now();
        let mut removable = self
            .state
            .iter()
            .filter_map(|entry| {
                let expired_lockout = entry.lockout_until.is_some_and(|until| now >= until);
                if entry.lockout_until.is_none() || expired_lockout {
                    Some((*entry.key(), entry.last_event))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        removable.sort_by_key(|(_, last_event)| *last_event);

        let remove_count = self.state.len().saturating_sub(Self::MAX_ENTRIES) + 1;
        for (ip, _) in removable.into_iter().take(remove_count) {
            self.state.remove(&ip);
        }
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
