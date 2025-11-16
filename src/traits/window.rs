/// Window dimensions
#[derive(Debug, Clone, Copy)]
pub struct WindowDimensions {
    pub width: u32,
    pub height: u32,
}

impl WindowDimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Window abstraction - handles display and drawing
pub trait WindowContext {
    /// Get window dimensions in physical pixels
    fn dimensions(&self) -> WindowDimensions;

    /// Draw pixels to the window
    fn draw(&self, pixels: &[u8]) -> Result<(), Box<dyn std::error::Error>>;

    /// Request the window to redraw
    fn request_redraw(&self);
}
