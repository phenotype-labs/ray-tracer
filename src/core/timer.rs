/// Self-contained timers - manage internal state, no Frame dependency
/// Each timer accumulates delta time and decides when to fire

/// Fixed rate timer - fires at specific Hz
#[derive(Debug, Clone, Copy)]
pub struct FixedHz {
    pub interval: f32,
    pub accumulator: f32,
}

impl FixedHz {
    /// Create timer that fires at given frequency
    pub fn new(hz: f32) -> Self {
        Self {
            interval: 1.0 / hz,
            accumulator: 0.0,
        }
    }

    /// Update with delta, returns true if should fire
    pub fn tick(&mut self, delta: f32) -> bool {
        self.accumulator += delta;

        if self.accumulator >= self.interval {
            self.accumulator -= self.interval;
            true
        } else {
            false
        }
    }

    /// Get interpolation alpha for smooth rendering
    pub fn alpha(&self) -> f32 {
        self.accumulator / self.interval
    }
}

/// Frame counter - fires every N ticks
#[derive(Debug, Clone, Copy)]
pub struct EveryNTicks {
    interval: u64,
    count: u64,
}

impl EveryNTicks {
    /// Create timer that fires every N ticks
    pub fn new(interval: u64) -> Self {
        Self { interval, count: 0 }
    }

    /// Tick once, returns true if should fire
    pub fn tick(&mut self) -> bool {
        self.count += 1;
        if self.count >= self.interval {
            self.count = 0;
            true
        } else {
            false
        }
    }

    /// Reset counter
    pub fn reset(&mut self) {
        self.count = 0;
    }
}

/// Physics accumulator - yields fixed timesteps for deterministic simulation
#[derive(Debug, Clone)]
pub struct Accumulator {
    timestep: f32,
    accumulator: f32,
    max_steps: u8,
}

impl Accumulator {
    /// Create accumulator with fixed timestep
    pub fn new(hz: f32, max_steps: u8) -> Self {
        Self {
            timestep: 1.0 / hz,
            accumulator: 0.0,
            max_steps,
        }
    }

    /// Update with delta, returns iterator of fixed timesteps to execute
    pub fn tick(&mut self, delta: f32) -> impl Iterator<Item = f32> {
        self.accumulator += delta;

        let steps = (self.accumulator / self.timestep)
            .min(self.max_steps as f32) as usize;

        self.accumulator -= steps as f32 * self.timestep;

        std::iter::repeat(self.timestep).take(steps)
    }

    /// Get interpolation alpha for rendering between physics steps
    pub fn alpha(&self) -> f32 {
        self.accumulator / self.timestep
    }
}

/// Throttled timer - minimum interval between fires
#[derive(Debug, Clone, Copy)]
pub struct Throttled {
    min_interval: f32,
    time_since_last: f32,
}

impl Throttled {
    /// Create throttled timer with minimum interval
    pub fn new(min_interval: f32) -> Self {
        Self {
            min_interval,
            time_since_last: min_interval, // Allow immediate first tick
        }
    }

    /// Attempt to fire, returns true if enough time has passed
    pub fn try_tick(&mut self, delta: f32) -> bool {
        self.time_since_last += delta;

        if self.time_since_last >= self.min_interval {
            self.time_since_last = 0.0;
            true
        } else {
            false
        }
    }
}

/// Countdown timer - fires once after duration
#[derive(Debug, Clone, Copy)]
pub struct Countdown {
    duration: f32,
    elapsed: f32,
    active: bool,
}

impl Countdown {
    /// Create inactive countdown
    pub fn new(duration: f32) -> Self {
        Self {
            duration,
            elapsed: 0.0,
            active: false,
        }
    }

    /// Start countdown
    pub fn start(&mut self) {
        self.elapsed = 0.0;
        self.active = true;
    }

    /// Tick with delta, returns true if completed
    pub fn tick(&mut self, delta: f32) -> bool {
        if !self.active {
            return false;
        }

        self.elapsed += delta;

        if self.elapsed >= self.duration {
            self.active = false;
            true
        } else {
            false
        }
    }

    /// Get progress [0, 1]
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_hz_fires_at_rate() {
        let mut timer = FixedHz::new(60.0); // 60Hz = 0.0166s

        // Small delta - no fire
        assert!(!timer.tick(0.01));

        // Accumulate to threshold
        assert!(timer.tick(0.01)); // Total ~0.02s >= 0.0166s

        // Immediate next - no fire
        assert!(!timer.tick(0.001));
    }

    #[test]
    fn every_n_ticks_counts() {
        let mut timer = EveryNTicks::new(3);

        assert!(!timer.tick()); // count=1
        assert!(!timer.tick()); // count=2
        assert!(timer.tick());  // count=3, fire and reset
        assert!(!timer.tick()); // count=1 again
    }

    #[test]
    fn accumulator_yields_fixed_steps() {
        let mut acc = Accumulator::new(60.0, 4);

        // Small delta - no steps
        let steps1: Vec<_> = acc.tick(0.01).collect();
        assert_eq!(steps1.len(), 0);

        // Enough for 1 step
        let steps2: Vec<_> = acc.tick(0.01).collect();
        assert_eq!(steps2.len(), 1);
        assert_eq!(steps2[0], 1.0 / 60.0);

        // Large delta - multiple steps (capped)
        let steps3: Vec<_> = acc.tick(0.1).collect();
        assert_eq!(steps3.len(), 4); // Capped at max_steps
    }

    #[test]
    fn throttled_enforces_minimum() {
        let mut timer = Throttled::new(0.1);

        assert!(timer.try_tick(0.05));  // First fire immediate
        assert!(!timer.try_tick(0.05)); // Too soon
        assert!(timer.try_tick(0.06));  // Enough time
    }

    #[test]
    fn countdown_fires_once() {
        let mut timer = Countdown::new(1.0);

        assert!(!timer.tick(0.5)); // Inactive

        timer.start();
        assert!(!timer.tick(0.5)); // In progress
        assert_eq!(timer.progress(), 0.5);

        assert!(timer.tick(0.6));  // Complete
        assert!(!timer.tick(0.1)); // Inactive again
    }
}
