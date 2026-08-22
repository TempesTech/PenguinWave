//! Per-connection request rate cap.

use std::time::Instant;

/// Sustained requests per second, with a burst allowance.
///
/// Generous by design: a slider drag is a legitimate burst. The cap exists so
/// one misbehaving client cannot starve the others, not to pace normal use.
pub const RATE_PER_SEC: f64 = 200.0;
pub const BURST: f64 = 400.0;

pub struct RateLimiter {
    tokens: f64,
    last: Instant,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            tokens: BURST,
            last: Instant::now(),
        }
    }

    pub fn allow(&mut self) -> bool {
        self.allow_at(Instant::now())
    }

    pub fn allow_at(&mut self, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + elapsed * RATE_PER_SEC).min(BURST);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_burst_is_allowed_then_refused() {
        let mut limiter = RateLimiter::new();
        let now = Instant::now();
        for _ in 0..BURST as usize {
            assert!(limiter.allow_at(now));
        }
        assert!(!limiter.allow_at(now));
    }

    #[test]
    fn tokens_refill_over_time() {
        let mut limiter = RateLimiter::new();
        let start = Instant::now();
        for _ in 0..BURST as usize {
            limiter.allow_at(start);
        }
        assert!(!limiter.allow_at(start));
        assert!(limiter.allow_at(start + Duration::from_secs(1)));
    }
}
