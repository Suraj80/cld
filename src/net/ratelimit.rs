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
        let interval_ns = self.refill_interval.as_nanos();

        if interval_ns == 0 {
            return;
        }

        let elapsed_ns = self.last_refill.elapsed().as_nanos();
        let tokens_to_add = elapsed_ns / interval_ns;

        if tokens_to_add == 0 {
            return;
        }

        let room = (self.capacity - self.tokens) as u128;
        let add = tokens_to_add.min(room) as u32;
        self.tokens += add;

        let consumed_ns = tokens_to_add.saturating_mul(interval_ns).min(u64::MAX as u128) as u64;
        self.last_refill += Duration::from_nanos(consumed_ns);
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
        limiter.last_refill = before_refill - Duration::from_millis(1100);

        limiter.refill();

        assert_eq!(limiter.tokens, 1);
        assert!(limiter.last_refill <= before_refill - Duration::from_millis(100));
    }
}
