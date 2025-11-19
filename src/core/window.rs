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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_dimensions_new() {
        let dims = WindowDimensions::new(1920, 1080);
        assert_eq!(dims.width, 1920);
        assert_eq!(dims.height, 1080);
    }

    #[test]
    fn test_window_dimensions_clone() {
        let dims1 = WindowDimensions::new(800, 600);
        let dims2 = dims1.clone();
        assert_eq!(dims1.width, dims2.width);
        assert_eq!(dims1.height, dims2.height);
    }

    #[test]
    fn test_window_dimensions_copy() {
        let dims1 = WindowDimensions::new(1024, 768);
        let dims2 = dims1; // Copy

        assert_eq!(dims1.width, 1024);
        assert_eq!(dims2.width, 1024);
    }

    #[test]
    fn test_window_dimensions_debug() {
        let dims = WindowDimensions::new(640, 480);
        let debug_str = format!("{:?}", dims);
        assert!(debug_str.contains("WindowDimensions"));
        assert!(debug_str.contains("640"));
        assert!(debug_str.contains("480"));
    }

    #[test]
    fn test_window_dimensions_various_sizes() {
        let test_cases = [
            (1, 1),
            (640, 480),
            (1280, 720),
            (1920, 1080),
            (3840, 2160),
        ];

        for (width, height) in test_cases {
            let dims = WindowDimensions::new(width, height);
            assert_eq!(dims.width, width);
            assert_eq!(dims.height, height);
        }
    }

    // Mock window for testing trait implementation
    struct MockWindow {
        dims: WindowDimensions,
        draw_called: std::cell::RefCell<usize>,
        redraw_called: std::cell::RefCell<usize>,
    }

    impl MockWindow {
        fn new(width: u32, height: u32) -> Self {
            Self {
                dims: WindowDimensions::new(width, height),
                draw_called: std::cell::RefCell::new(0),
                redraw_called: std::cell::RefCell::new(0),
            }
        }

        fn draw_call_count(&self) -> usize {
            *self.draw_called.borrow()
        }

        fn redraw_call_count(&self) -> usize {
            *self.redraw_called.borrow()
        }
    }

    impl WindowContext for MockWindow {
        fn dimensions(&self) -> WindowDimensions {
            self.dims
        }

        fn draw(&self, _pixels: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
            *self.draw_called.borrow_mut() += 1;
            Ok(())
        }

        fn request_redraw(&self) {
            *self.redraw_called.borrow_mut() += 1;
        }
    }

    #[test]
    fn test_window_context_dimensions() {
        let window = MockWindow::new(1920, 1080);
        let dims = window.dimensions();
        assert_eq!(dims.width, 1920);
        assert_eq!(dims.height, 1080);
    }

    #[test]
    fn test_window_context_draw() {
        let window = MockWindow::new(100, 100);
        let pixels = vec![0u8; 100 * 100 * 4];

        assert_eq!(window.draw_call_count(), 0);

        let result = window.draw(&pixels);
        assert!(result.is_ok());
        assert_eq!(window.draw_call_count(), 1);

        let result = window.draw(&pixels);
        assert!(result.is_ok());
        assert_eq!(window.draw_call_count(), 2);
    }

    #[test]
    fn test_window_context_redraw() {
        let window = MockWindow::new(800, 600);

        assert_eq!(window.redraw_call_count(), 0);

        window.request_redraw();
        assert_eq!(window.redraw_call_count(), 1);

        window.request_redraw();
        window.request_redraw();
        assert_eq!(window.redraw_call_count(), 3);
    }

    #[test]
    fn test_window_context_draw_empty_buffer() {
        let window = MockWindow::new(10, 10);
        let empty_pixels: Vec<u8> = vec![];

        let result = window.draw(&empty_pixels);
        assert!(result.is_ok());
        assert_eq!(window.draw_call_count(), 1);
    }

    #[test]
    fn test_window_context_multiple_operations() {
        let window = MockWindow::new(640, 480);
        let pixels = vec![128u8; 640 * 480 * 4];

        // Interleave draw and redraw calls
        let _ = window.draw(&pixels);
        window.request_redraw();
        let _ = window.draw(&pixels);
        window.request_redraw();
        window.request_redraw();

        assert_eq!(window.draw_call_count(), 2);
        assert_eq!(window.redraw_call_count(), 3);
    }
}
