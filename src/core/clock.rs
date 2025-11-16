use std::time::Instant;

/// Minimal game clock - just tracks delta time
/// Systems manage their own internal state
#[derive(Debug)]
pub struct Clock {
    last_tick: Instant,
}

impl Clock {
    /// Create new clock starting now
    pub fn new() -> Self {
        Self {
            last_tick: Instant::now(),
        }
    }

    /// Get delta time since last tick and advance clock
    /// Returns delta in seconds
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let delta = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;
        delta
    }

    /// Reset clock to current time
    pub fn reset(&mut self) {
        self.last_tick = Instant::now();
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn clock_measures_delta() {
        let mut clock = Clock::new();

        thread::sleep(Duration::from_millis(10));
        let delta = clock.tick();

        // Should be roughly 10ms = 0.01s
        assert!(delta >= 0.009 && delta <= 0.020);
    }

    #[test]
    fn clock_resets() {
        let mut clock = Clock::new();

        thread::sleep(Duration::from_millis(10));
        clock.reset();

        let delta = clock.tick();
        // Should be very small since we just reset
        assert!(delta < 0.005);
    }
}
