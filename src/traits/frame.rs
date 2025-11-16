use crate::frame::FrameInfo;

/// Frame timing and iteration abstraction
pub trait FrameSource: Iterator<Item = FrameInfo> {
    /// Get the delta time since last frame in seconds
    fn delta_time(&self) -> f32;

    /// Get the total number of frames processed
    fn frame_count(&self) -> u64;
}
