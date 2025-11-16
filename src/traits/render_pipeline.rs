use super::controller::Controller;
use super::display_context::DisplayContext;
use super::frame::Frame;

/// Manages layer composition and scene rendering
pub trait RenderPipeline {
    /// Update all layers based on frame timing
    fn update(&mut self, frame: &Frame, controller: &dyn Controller);

    /// Render all layers and compose final frame pixels
    /// Returns RGBA pixel data for the given display context
    fn render(&self, context: &DisplayContext) -> Vec<u8>;
}
