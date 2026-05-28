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
        let elapsed = self.last_refill.elapsed();

        let tokens_to_add = elapsed.as_secs() / self.refill_interval.as_secs();

        if tokens_to_add == 0 {
            return;
        }

        self.tokens = (self.tokens + tokens_to_add as u32).min(self.capacity);
        self.last_refill = Instant::now();
    }
}
