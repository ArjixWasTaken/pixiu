//! Throttles: how often a client, or an account, may try something within a
//! sliding window. Wrong passwords, password reset requests, registrations
//! and single sign-on starts count; past the limit the API answers 429.

use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{StatusCode, request::Parts},
};

use crate::{ApiError, ApiState};

/// What is throttled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Action {
    /// Signing in with a wrong password, per client.
    FailedLogin,
    /// Asking for a password reset, per client.
    Forgot,
    /// Reset emails, per account: more are not sent, silently.
    ResetEmail,
    /// Confirmation emails asked for again, per account.
    VerifyEmail,
    /// Asking for an account, per client.
    Register,
}

impl Action {
    /// How many within how long.
    fn limit(self) -> (usize, Duration) {
        const MINUTE: Duration = Duration::from_secs(60);
        match self {
            Self::FailedLogin => (20, 15 * MINUTE),
            Self::Forgot => (10, 60 * MINUTE),
            Self::ResetEmail | Self::VerifyEmail => (3, 60 * MINUTE),
            Self::Register => (5, 60 * MINUTE),
        }
    }
}

/// Past this many keys, idle ones are forgotten.
const PRUNE_AFTER: usize = 10_000;

#[derive(Default)]
pub struct Throttle {
    hits: Mutex<HashMap<(Action, String), VecDeque<Instant>>>,
}

impl Throttle {
    /// Whether `key` is at its limit for `action` now; counts nothing.
    pub(crate) fn blocked(&self, action: Action, key: &str) -> bool {
        let (limit, window) = action.limit();
        let mut hits = self.hits.lock().unwrap();
        hits.get_mut(&(action, key.to_owned()))
            .is_some_and(|times| recent(times, window) >= limit)
    }

    /// Counts one `action` by `key`.
    pub(crate) fn hit(&self, action: Action, key: &str) {
        let (_, window) = action.limit();
        let mut hits = self.hits.lock().unwrap();
        if hits.len() > PRUNE_AFTER {
            hits.retain(|(action, _), times| recent(times, action.limit().1) > 0);
        }
        let times = hits.entry((action, key.to_owned())).or_default();
        recent(times, window);
        times.push_back(Instant::now());
    }

    /// Counts one `action` by `key`, if it is under its limit.
    pub(crate) fn allow(&self, action: Action, key: &str) -> bool {
        if self.blocked(action, key) {
            return false;
        }
        self.hit(action, key);
        true
    }

    /// [`allow`](Self::allow), answering 429 when not.
    pub(crate) fn check(&self, action: Action, key: &str) -> Result<(), ApiError> {
        if self.allow(action, key) {
            Ok(())
        } else {
            Err(too_many())
        }
    }
}

/// Drops the times that left the window; how many are left.
fn recent(times: &mut VecDeque<Instant>, window: Duration) -> usize {
    while times.front().is_some_and(|time| time.elapsed() >= window) {
        times.pop_front();
    }
    times.len()
}

pub(crate) fn too_many() -> ApiError {
    ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "Too many tries; wait a while and try again.",
    )
}

/// The address a request came from: the connecting peer, or, behind a
/// trusted reverse proxy, the last address in `X-Forwarded-For` (the one the
/// proxy added).
pub(crate) struct ClientIp(pub String);

impl FromRequestParts<ApiState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        if state.trust_proxy_headers
            && let Some(ip) = parts
                .headers
                .get_all("x-forwarded-for")
                .iter()
                .filter_map(|value| value.to_str().ok())
                .flat_map(|value| value.split(','))
                .next_back()
                .and_then(|ip| ip.trim().parse::<IpAddr>().ok())
        {
            return Ok(Self(ip.to_string()));
        }
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map_or_else(|| "unknown".to_owned(), |info| info.0.ip().to_string());
        Ok(Self(peer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_hold_per_key_and_action() {
        let throttle = Throttle::default();
        for _ in 0..3 {
            assert!(throttle.allow(Action::ResetEmail, "1"));
        }
        assert!(!throttle.allow(Action::ResetEmail, "1"));
        assert!(throttle.blocked(Action::ResetEmail, "1"));
        assert!(throttle.allow(Action::ResetEmail, "2"));
        assert!(throttle.allow(Action::VerifyEmail, "1"));
        assert!(!throttle.blocked(Action::FailedLogin, "1"));
    }

    #[test]
    fn old_hits_leave_the_window() {
        let throttle = Throttle::default();
        let long_ago = Instant::now()
            .checked_sub(Duration::from_secs(2 * 60 * 60))
            .unwrap();
        throttle
            .hits
            .lock()
            .unwrap()
            .insert((Action::ResetEmail, "1".to_owned()), [long_ago; 3].into());
        assert!(throttle.allow(Action::ResetEmail, "1"));
        assert_eq!(
            throttle.hits.lock().unwrap()[&(Action::ResetEmail, "1".to_owned())].len(),
            1
        );
    }
}
