use super::window::WindowDimensions;
use super::frame::Frame;

/// Pipeline executor - orchestrates rendering and provides frame iteration
pub trait PipelineExecutor {
    /// Register window dimensions for rendering
    fn register_window_dimensions(&mut self, dimensions: WindowDimensions);

    /// Get an iterator over frames with pixels
    fn frames(&mut self) -> Box<dyn Iterator<Item = Frame> + '_>;
}
