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
        let refill_millis = self.refill_interval.as_millis();
        if refill_millis == 0 {
            return;
        }

        let elapsed = self.last_refill.elapsed();
        let tokens_to_add = elapsed.as_millis() / refill_millis;

        if tokens_to_add == 0 {
            return;
        }

        let tokens_to_add = u32::try_from(tokens_to_add).unwrap_or(u32::MAX);
        self.tokens = self.tokens.saturating_add(tokens_to_add).min(self.capacity);
        self.last_refill += self.refill_interval * tokens_to_add;
    }
}
