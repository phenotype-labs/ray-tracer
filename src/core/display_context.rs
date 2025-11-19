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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_context_with_dimensions() {
        let ctx = DisplayContext::new(1920, 1080);
        assert_eq!(ctx.width, 1920);
        assert_eq!(ctx.height, 1080);
    }

    #[test]
    fn test_pixel_count_calculation() {
        let ctx = DisplayContext::new(640, 480);
        assert_eq!(ctx.pixel_count(), 640 * 480);
        assert_eq!(ctx.pixel_count(), 307200);
    }

    #[test]
    fn test_buffer_size_rgba() {
        let ctx = DisplayContext::new(100, 100);
        // 100x100 pixels * 4 bytes per pixel (RGBA)
        assert_eq!(ctx.buffer_size(), 40000);
    }

    #[test]
    fn test_small_dimensions() {
        let ctx = DisplayContext::new(1, 1);
        assert_eq!(ctx.pixel_count(), 1);
        assert_eq!(ctx.buffer_size(), 4);
    }

    #[test]
    fn test_large_dimensions() {
        let ctx = DisplayContext::new(3840, 2160); // 4K
        assert_eq!(ctx.pixel_count(), 8294400);
        assert_eq!(ctx.buffer_size(), 33177600);
    }

    #[test]
    fn test_non_square_dimensions() {
        let ctx = DisplayContext::new(1024, 768);
        assert_eq!(ctx.pixel_count(), 786432);
        assert_eq!(ctx.buffer_size(), 3145728);
    }

    #[test]
    fn test_clone() {
        let ctx1 = DisplayContext::new(800, 600);
        let ctx2 = ctx1.clone();
        assert_eq!(ctx1.width, ctx2.width);
        assert_eq!(ctx1.height, ctx2.height);
        assert_eq!(ctx1.pixel_count(), ctx2.pixel_count());
    }

    #[test]
    fn test_copy_semantics() {
        let ctx1 = DisplayContext::new(1280, 720);
        let ctx2 = ctx1; // Copy, not move

        // Both should be usable
        assert_eq!(ctx1.width, 1280);
        assert_eq!(ctx2.width, 1280);
    }

    #[test]
    fn test_debug_format() {
        let ctx = DisplayContext::new(1024, 768);
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("DisplayContext"));
        assert!(debug_str.contains("1024"));
        assert!(debug_str.contains("768"));
    }

    #[test]
    fn test_buffer_size_relationship() {
        let ctx = DisplayContext::new(200, 150);
        // buffer_size should always be pixel_count * 4
        assert_eq!(ctx.buffer_size(), ctx.pixel_count() * 4);
    }

    #[test]
    fn test_various_common_resolutions() {
        let resolutions = [
            (640, 480),    // VGA
            (800, 600),    // SVGA
            (1024, 768),   // XGA
            (1280, 720),   // HD
            (1920, 1080),  // Full HD
            (2560, 1440),  // QHD
            (3840, 2160),  // 4K UHD
        ];

        for (width, height) in resolutions {
            let ctx = DisplayContext::new(width, height);
            assert_eq!(ctx.width, width);
            assert_eq!(ctx.height, height);
            assert_eq!(ctx.pixel_count(), (width * height) as usize);
            assert_eq!(ctx.buffer_size(), (width * height * 4) as usize);
        }
    }
}
