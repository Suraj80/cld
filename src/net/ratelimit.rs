use std::time::{Duration, Instant};

pub struct RateLimiter {
    capacity: u32,
    tokens: u32,
    refill_interval: Duration,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(capacity: u32, refill_interval: Duration) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_interval,
            last_refill: Instant::now(),
        }
    }

    pub fn allow(&mut self) -> bool {
        self.refill();

        if self.tokens == 0 {
            return false;
        }

        self.tokens -= 1;
        true
    }

    fn refill(&mut self) {
        let interval_ms = self.refill_interval.as_millis();

        if interval_ms == 0 {
            return;
        }

        let elapsed_ms = self.last_refill.elapsed().as_millis();
        let tokens_to_add = elapsed_ms / interval_ms;

        if tokens_to_add == 0 {
            return;
        }

        self.tokens = (self.tokens + tokens_to_add as u32).min(self.capacity);
        self.last_refill += Duration::from_millis((tokens_to_add * interval_ms) as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refill_preserves_sub_second_remainder() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
        limiter.tokens = 0;

        let before_refill = Instant::now();
        limiter.last_refill = before_refill - Duration::from_millis(1900);

        limiter.refill();

        assert_eq!(limiter.tokens, 1);
        assert!(limiter.last_refill <= before_refill - Duration::from_millis(800));
    }
}
