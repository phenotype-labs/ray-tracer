/// Frame metadata - carries frame number and timing info
#[derive(Debug, Clone, Copy)]
pub struct FrameInfo {
    pub number: u64,
    pub time: f32,
    pub delta: f32,
}

impl FrameInfo {
    pub fn new(number: u64, time: f32, delta: f32) -> Self {
        Self { number, time, delta }
    }
}

/// Infinite iterator that yields frame information
/// Use this in a loop: `for frame in frames { ... }`
pub struct FrameIterator {
    frame_number: u64,
    start_time: std::time::Instant,
    last_frame_time: std::time::Instant,
}

impl FrameIterator {
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            frame_number: 0,
            start_time: now,
            last_frame_time: now,
        }
    }

    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    pub fn time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }
}

impl Default for FrameIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for FrameIterator {
    type Item = FrameInfo;

    fn next(&mut self) -> Option<FrameInfo> {
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_frame_time).as_secs_f32();
        let time = now.duration_since(self.start_time).as_secs_f32();

        let info = FrameInfo::new(self.frame_number, time, delta);

        self.frame_number += 1;
        self.last_frame_time = now;

        Some(info)
    }
}
