use super::window::WindowContext;

/// Window renderer - simple trait for displaying pixels to a window
pub trait WindowRenderer {
    /// Register window context for rendering
    fn register_window(&mut self, window: &dyn WindowContext);

    /// Render pixels to the registered window
    fn render(&self, pixels: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}
