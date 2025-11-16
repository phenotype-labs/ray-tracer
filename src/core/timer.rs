use super::frame::Frame;

/// Zero-cost timer abstraction for multi-rate updates
/// Designed for cache efficiency and inline optimization
pub trait Timer {
    /// Returns true if timer should fire this frame
    /// MUST be inline-friendly (no allocations, simple math)
    fn should_tick(&self, frame: &Frame) -> bool;

    /// Update internal state after tick (for stateful timers)
    fn consume(&mut self, _frame: &Frame) {}
}

/// Fixed frequency timer - fires at specific Hz (physics, network sync)
///
/// Example: 60Hz physics updates, 20Hz network sync
/// Memory: 8 bytes (cache-line friendly)
#[derive(Debug, Clone, Copy)]
pub struct FixedHz {
    pub interval: f32,
    pub last_tick: f32,
}

impl FixedHz {
    /// Create timer that fires at given frequency
    ///
    /// # Examples
    /// ```
    /// let physics_timer = FixedHz::new(60.0);  // 60Hz
    /// let network_timer = FixedHz::new(20.0);  // 20Hz
    /// ```
    #[inline]
    pub fn new(hz: f32) -> Self {
        Self {
            interval: 1.0 / hz,
            last_tick: 0.0,
        }
    }

    /// Create timer with specific interval in seconds
    #[inline]
    pub fn from_interval(interval: f32) -> Self {
        Self {
            interval,
            last_tick: 0.0,
        }
    }
}

impl Timer for FixedHz {
    #[inline(always)]
    fn should_tick(&self, frame: &Frame) -> bool {
        frame.time - self.last_tick >= self.interval
    }

    #[inline(always)]
    fn consume(&mut self, frame: &Frame) {
        self.last_tick = frame.time;
    }
}

/// Frame-based timer - fires every N frames (diagnostics, autosave)
///
/// Branch-predictor friendly for regular intervals
/// Memory: 8 bytes
#[derive(Debug, Clone, Copy)]
pub struct EveryNFrames {
    pub every_n: u64,
}

impl EveryNFrames {
    /// Create timer that fires every N frames
    ///
    /// # Examples
    /// ```
    /// let autosave = EveryNFrames::new(144 * 60);  // Every 60s at 144fps
    /// let diagnostics = EveryNFrames::new(144);     // Every 1s at 144fps
    /// ```
    #[inline]
    pub fn new(every_n: u64) -> Self {
        Self { every_n }
    }
}

impl Timer for EveryNFrames {
    #[inline(always)]
    fn should_tick(&self, frame: &Frame) -> bool {
        frame.number % self.every_n == 0
    }
}

/// Fixed timestep accumulator - executes multiple steps per frame if needed
///
/// Essential for stable physics simulation (Gauss-Seidel, constraint solving)
/// Prevents timestep-dependent behavior and spiral of death
/// Memory: 12 bytes
#[derive(Debug, Clone)]
pub struct Accumulator {
    pub timestep: f32,
    pub accumulator: f32,
    pub max_steps: u8,  // Safety limit to prevent spiral of death
}

impl Accumulator {
    /// Create accumulator with fixed timestep
    ///
    /// # Arguments
    /// * `hz` - Update frequency (e.g., 60.0 for 60Hz physics)
    /// * `max_steps` - Maximum steps per frame (prevents spiral of death)
    ///
    /// # Examples
    /// ```
    /// let physics = Accumulator::new(60.0, 4);  // 60Hz, max 4 steps
    /// ```
    #[inline]
    pub fn new(hz: f32, max_steps: u8) -> Self {
        Self {
            timestep: 1.0 / hz,
            accumulator: 0.0,
            max_steps,
        }
    }

    /// Returns iterator of timesteps to execute this frame
    ///
    /// Consumes accumulated time and returns 0-max_steps iterations
    /// Each iteration gets a fixed timestep for deterministic simulation
    ///
    /// # Examples
    /// ```
    /// for dt in physics_accumulator.tick(frame) {
    ///     integrate_velocities(dt);
    ///     solve_constraints(dt);  // Gauss-Seidel iterations
    ///     apply_damping(dt);
    /// }
    /// ```
    #[inline]
    pub fn tick(&mut self, frame: &Frame) -> impl Iterator<Item = f32> {
        self.accumulator += frame.delta;

        let steps = (self.accumulator / self.timestep)
            .min(self.max_steps as f32) as usize;

        self.accumulator -= steps as f32 * self.timestep;

        std::iter::repeat(self.timestep).take(steps)
    }

    /// Get interpolation alpha for rendering between physics steps
    ///
    /// Returns value in [0, 1] for smooth visual interpolation
    #[inline]
    pub fn alpha(&self) -> f32 {
        self.accumulator / self.timestep
    }
}

/// Throttled timer - minimum interval between ticks (rate limiting, debouncing)
///
/// Fires at most once per interval, useful for input debouncing
/// Memory: 8 bytes
#[derive(Debug, Clone, Copy)]
pub struct Throttled {
    pub min_interval: f32,
    pub last_tick: f32,
}

impl Throttled {
    /// Create throttled timer with minimum interval
    ///
    /// # Examples
    /// ```
    /// let input_debounce = Throttled::new(0.1);  // Max 10Hz
    /// ```
    #[inline]
    pub fn new(min_interval: f32) -> Self {
        Self {
            min_interval,
            last_tick: -min_interval,  // Allow immediate first tick
        }
    }

    /// Attempt to tick, returns true if enough time has passed
    #[inline]
    pub fn try_tick(&mut self, frame: &Frame) -> bool {
        if frame.time - self.last_tick >= self.min_interval {
            self.last_tick = frame.time;
            true
        } else {
            false
        }
    }
}

/// Countdown timer - fires once after specified duration
///
/// Useful for delayed actions, cooldowns
/// Memory: 12 bytes
#[derive(Debug, Clone, Copy)]
pub struct Countdown {
    pub duration: f32,
    pub elapsed: f32,
    pub active: bool,
}

impl Countdown {
    /// Create inactive countdown timer
    #[inline]
    pub fn new(duration: f32) -> Self {
        Self {
            duration,
            elapsed: 0.0,
            active: false,
        }
    }

    /// Start the countdown
    #[inline]
    pub fn start(&mut self) {
        self.elapsed = 0.0;
        self.active = true;
    }

    /// Update and check if countdown completed
    #[inline]
    pub fn tick(&mut self, frame: &Frame) -> bool {
        if !self.active {
            return false;
        }

        self.elapsed += frame.delta;

        if self.elapsed >= self.duration {
            self.active = false;
            true
        } else {
            false
        }
    }

    /// Get progress in [0, 1]
    #[inline]
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).min(1.0)
    }
}

/// Timer combinator - AND logic (both must fire)
#[derive(Debug, Clone)]
pub struct AndTimer<A: Timer, B: Timer> {
    pub a: A,
    pub b: B,
}

impl<A: Timer, B: Timer> Timer for AndTimer<A, B> {
    #[inline]
    fn should_tick(&self, frame: &Frame) -> bool {
        self.a.should_tick(frame) && self.b.should_tick(frame)
    }

    #[inline]
    fn consume(&mut self, frame: &Frame) {
        self.a.consume(frame);
        self.b.consume(frame);
    }
}

/// Timer combinator - OR logic (either can fire)
#[derive(Debug, Clone)]
pub struct OrTimer<A: Timer, B: Timer> {
    pub a: A,
    pub b: B,
}

impl<A: Timer, B: Timer> Timer for OrTimer<A, B> {
    #[inline]
    fn should_tick(&self, frame: &Frame) -> bool {
        self.a.should_tick(frame) || self.b.should_tick(frame)
    }

    #[inline]
    fn consume(&mut self, frame: &Frame) {
        self.a.consume(frame);
        self.b.consume(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_frame(number: u64, time: f32, delta: f32) -> Frame {
        Frame::new(number, time, delta, vec![])
    }

    #[test]
    fn fixed_hz_fires_at_correct_rate() {
        let mut timer = FixedHz::new(60.0);  // 60Hz = 0.0166s interval

        // Should not fire at time 0 (last_tick defaults to 0)
        let frame1 = test_frame(0, 0.0, 0.0);
        assert!(!timer.should_tick(&frame1));

        // Should fire after interval
        let frame2 = test_frame(1, 0.017, 0.017);
        assert!(timer.should_tick(&frame2));
        timer.consume(&frame2);

        // Should not fire immediately after consumption
        let frame3 = test_frame(2, 0.020, 0.003);
        assert!(!timer.should_tick(&frame3));

        // Should fire again after another interval
        let frame4 = test_frame(3, 0.034, 0.014);
        assert!(timer.should_tick(&frame4));
    }

    #[test]
    fn every_n_frames_fires_correctly() {
        let timer = EveryNFrames::new(10);

        assert!(timer.should_tick(&test_frame(0, 0.0, 0.0)));
        assert!(!timer.should_tick(&test_frame(1, 0.016, 0.016)));
        assert!(!timer.should_tick(&test_frame(9, 0.144, 0.016)));
        assert!(timer.should_tick(&test_frame(10, 0.160, 0.016)));
        assert!(timer.should_tick(&test_frame(20, 0.320, 0.016)));
    }

    #[test]
    fn accumulator_handles_multiple_steps() {
        let mut acc = Accumulator::new(60.0, 4);

        // Small delta - no steps
        let frame1 = test_frame(0, 0.0, 0.01);
        let steps1: Vec<_> = acc.tick(&frame1).collect();
        assert_eq!(steps1.len(), 0);

        // Accumulated enough for 1 step
        let frame2 = test_frame(1, 0.01, 0.01);
        let steps2: Vec<_> = acc.tick(&frame2).collect();
        assert_eq!(steps2.len(), 1);
        assert_eq!(steps2[0], 1.0 / 60.0);

        // Large delta - multiple steps (capped at max_steps)
        let frame3 = test_frame(2, 0.12, 0.1);
        let steps3: Vec<_> = acc.tick(&frame3).collect();
        assert_eq!(steps3.len(), 4);  // Capped at max_steps
    }

    #[test]
    fn throttled_enforces_minimum_interval() {
        let mut timer = Throttled::new(0.1);

        let frame1 = test_frame(0, 0.0, 0.0);
        assert!(timer.try_tick(&frame1));  // First tick allowed

        let frame2 = test_frame(1, 0.05, 0.05);
        assert!(!timer.try_tick(&frame2));  // Too soon

        let frame3 = test_frame(2, 0.11, 0.06);
        assert!(timer.try_tick(&frame3));  // Enough time passed
    }

    #[test]
    fn countdown_completes_once() {
        let mut timer = Countdown::new(1.0);

        assert!(!timer.tick(&test_frame(0, 0.0, 0.0)));  // Inactive

        timer.start();
        assert!(!timer.tick(&test_frame(1, 0.5, 0.5)));  // In progress
        assert_eq!(timer.progress(), 0.5);

        assert!(timer.tick(&test_frame(2, 1.5, 1.0)));  // Completed
        assert!(!timer.tick(&test_frame(3, 2.0, 0.5)));  // Inactive again
    }
}
