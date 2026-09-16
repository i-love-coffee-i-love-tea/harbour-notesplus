//! In-memory rate limiting for HTTP server endpoints.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rate limit tracking for an individual client/key.
#[derive(Debug, Clone)]
struct ClientHistory {
    timestamps: Vec<Instant>,
    window: Duration,
}

/// Thread-safe in-memory rate limiter using sliding window algorithm.
#[derive(Clone, Default)]
pub struct RateLimiter {
    limits: Arc<Mutex<HashMap<(String, String), ClientHistory>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Checks if a request for `action` from `client_ip` is allowed under `max_requests` per `window`.
    /// Returns `Ok(())` if allowed, or `Err(retry_after_secs)` if rate limited.
    pub fn check(
        &self,
        action: &str,
        client_ip: &str,
        max_requests: usize,
        window: Duration,
    ) -> Result<(), u64> {
        let mut map = self.limits.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        // Periodically cleanup stale entries using each entry's own window duration
        if map.len() > 200 {
            map.retain(|_, history| {
                let entry_window = history.window;
                history.timestamps.retain(|&t| now.duration_since(t) < entry_window);
                !history.timestamps.is_empty()
            });
        }

        let key = (action.to_string(), client_ip.to_string());
        let history = map.entry(key).or_insert_with(|| ClientHistory {
            timestamps: Vec::new(),
            window,
        });
        history.window = window;

        // Prune expired timestamps for this key
        history
            .timestamps
            .retain(|&t| now.duration_since(t) < window);

        if history.timestamps.len() >= max_requests {
            let oldest = history.timestamps.first().copied().unwrap_or(now);
            let elapsed = now.duration_since(oldest);
            let retry_after = if elapsed < window {
                (window - elapsed).as_secs() + 1
            } else {
                1
            };
            Err(retry_after)
        } else {
            history.timestamps.push(now);
            Ok(())
        }
    }

    /// Clears all recorded rate limit history (useful for testing).
    pub fn clear(&self) {
        let mut map = self.limits.lock().unwrap_or_else(|e| e.into_inner());
        map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new();
        let ip = "192.168.1.50";
        for _ in 0..5 {
            assert!(limiter.check("test_action", ip, 5, Duration::from_secs(10)).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_exceeding_limit() {
        let limiter = RateLimiter::new();
        let ip = "192.168.1.51";
        for _ in 0..3 {
            assert!(limiter.check("auth_action", ip, 3, Duration::from_secs(10)).is_ok());
        }
        let err = limiter.check("auth_action", ip, 3, Duration::from_secs(10));
        assert!(err.is_err());
        let retry_after = err.unwrap_err();
        assert!(retry_after >= 1 && retry_after <= 10);
    }

    #[test]
    fn test_rate_limiter_isolates_different_ips_and_actions() {
        let limiter = RateLimiter::new();
        let ip1 = "10.0.0.1";
        let ip2 = "10.0.0.2";

        // Exhaust IP 1
        for _ in 0..2 {
            assert!(limiter.check("action1", ip1, 2, Duration::from_secs(10)).is_ok());
        }
        assert!(limiter.check("action1", ip1, 2, Duration::from_secs(10)).is_err());

        // IP 2 is still allowed
        assert!(limiter.check("action1", ip2, 2, Duration::from_secs(10)).is_ok());

        // IP 1 with different action is still allowed
        assert!(limiter.check("action2", ip1, 2, Duration::from_secs(10)).is_ok());
    }
}
