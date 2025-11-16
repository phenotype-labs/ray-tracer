/// Display context - contains rendering dimensions and metadata
#[derive(Debug, Clone, Copy)]
pub struct DisplayContext {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl DisplayContext {
    /// Create new display context
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Total number of pixels
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Total size in bytes for RGBA buffer
    pub fn buffer_size(&self) -> usize {
        self.pixel_count() * 4
    }
}
