use super::controller::Controller;
use super::display_context::DisplayContext;
use super::layer::{Layer, LayerLogic, LayerOutput, TimedLayer};

/// 2D drawing operations for canvas
#[derive(Debug, Clone, PartialEq)]
pub enum DrawOp {
    /// Fill entire canvas with color (r, g, b, a)
    Clear(u8, u8, u8, u8),

    /// Draw pixel at (x, y) with color (r, g, b, a)
    Pixel { x: u32, y: u32, r: u8, g: u8, b: u8, a: u8 },

    /// Draw horizontal line from (x, y) with length and color
    HLine { x: u32, y: u32, length: u32, r: u8, g: u8, b: u8, a: u8 },

    /// Draw vertical line from (x, y) with length and color
    VLine { x: u32, y: u32, length: u32, r: u8, g: u8, b: u8, a: u8 },

    /// Draw filled rectangle (x, y, width, height, r, g, b, a)
    Rect { x: u32, y: u32, width: u32, height: u32, r: u8, g: u8, b: u8, a: u8 },

    /// Draw circle at (cx, cy) with radius and color
    Circle { cx: u32, cy: u32, radius: u32, r: u8, g: u8, b: u8, a: u8 },

    /// Draw filled circle at (cx, cy) with radius and color
    FilledCircle { cx: u32, cy: u32, radius: u32, r: u8, g: u8, b: u8, a: u8 },

    /// Draw line from (x1, y1) to (x2, y2) with color
    Line { x1: u32, y1: u32, x2: u32, y2: u32, r: u8, g: u8, b: u8, a: u8 },
}

/// Canvas state - pixel buffer with draw operations
#[derive(Clone)]
pub struct Canvas {
    /// RGBA pixel buffer
    pixels: Vec<u8>,
    /// Alpha channel (0.0 = transparent, 1.0 = opaque)
    alpha: Vec<f32>,
    /// Pending draw operations
    operations: Vec<DrawOp>,
    /// Canvas dimensions
    width: u32,
    height: u32,
}

impl Canvas {
    /// Create new canvas with dimensions
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        let pixel_count = (width * height) as usize;

        Self {
            pixels: vec![0; size],
            alpha: vec![0.0; pixel_count],
            operations: Vec::new(),
            width,
            height,
        }
    }

    /// Add draw operation - functional style
    pub fn draw(mut self, op: DrawOp) -> Self {
        self.operations.push(op);
        self
    }

    /// Execute all pending operations and return new canvas
    pub fn execute_ops(&self) -> Self {
        let mut canvas = Self {
            pixels: self.pixels.clone(),
            alpha: self.alpha.clone(),
            operations: Vec::new(),
            width: self.width,
            height: self.height,
        };

        for op in &self.operations {
            canvas.execute_op(op);
        }

        canvas
    }

    /// Execute single draw operation (mutates internal state)
    fn execute_op(&mut self, op: &DrawOp) {
        match op {
            DrawOp::Clear(r, g, b, a) => self.clear(*r, *g, *b, *a),
            DrawOp::Pixel { x, y, r, g, b, a } => self.set_pixel(*x, *y, *r, *g, *b, *a),
            DrawOp::HLine { x, y, length, r, g, b, a } => self.draw_hline(*x, *y, *length, *r, *g, *b, *a),
            DrawOp::VLine { x, y, length, r, g, b, a } => self.draw_vline(*x, *y, *length, *r, *g, *b, *a),
            DrawOp::Rect { x, y, width, height, r, g, b, a } => {
                self.draw_rect(*x, *y, *width, *height, *r, *g, *b, *a)
            }
            DrawOp::Circle { cx, cy, radius, r, g, b, a } => {
                self.draw_circle(*cx, *cy, *radius, *r, *g, *b, *a)
            }
            DrawOp::FilledCircle { cx, cy, radius, r, g, b, a } => {
                self.draw_filled_circle(*cx, *cy, *radius, *r, *g, *b, *a)
            }
            DrawOp::Line { x1, y1, x2, y2, r, g, b, a } => {
                self.draw_line(*x1, *y1, *x2, *y2, *r, *g, *b, *a)
            }
        }
    }

    /// Clear canvas to color
    fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        let alpha_val = a as f32 / 255.0;

        for i in 0..self.width * self.height {
            let idx = (i * 4) as usize;
            self.pixels[idx] = r;
            self.pixels[idx + 1] = g;
            self.pixels[idx + 2] = b;
            self.pixels[idx + 3] = a;
            self.alpha[i as usize] = alpha_val;
        }
    }

    /// Set single pixel
    fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }

        let idx = ((y * self.width + x) * 4) as usize;
        let alpha_idx = (y * self.width + x) as usize;

        self.pixels[idx] = r;
        self.pixels[idx + 1] = g;
        self.pixels[idx + 2] = b;
        self.pixels[idx + 3] = a;
        self.alpha[alpha_idx] = a as f32 / 255.0;
    }

    /// Draw horizontal line
    fn draw_hline(&mut self, x: u32, y: u32, length: u32, r: u8, g: u8, b: u8, a: u8) {
        for i in 0..length {
            self.set_pixel(x + i, y, r, g, b, a);
        }
    }

    /// Draw vertical line
    fn draw_vline(&mut self, x: u32, y: u32, length: u32, r: u8, g: u8, b: u8, a: u8) {
        for i in 0..length {
            self.set_pixel(x, y + i, r, g, b, a);
        }
    }

    /// Draw filled rectangle
    fn draw_rect(&mut self, x: u32, y: u32, width: u32, height: u32, r: u8, g: u8, b: u8, a: u8) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, r, g, b, a);
            }
        }
    }

    /// Draw circle outline using midpoint circle algorithm
    fn draw_circle(&mut self, cx: u32, cy: u32, radius: u32, r: u8, g: u8, b: u8, a: u8) {
        let (mut x, mut y) = (radius as i32, 0i32);
        let mut p = 1 - radius as i32;

        let plot = |canvas: &mut Canvas, cx: i32, cy: i32, x: i32, y: i32| {
            let points = [
                (cx + x, cy + y), (cx - x, cy + y),
                (cx + x, cy - y), (cx - x, cy - y),
                (cx + y, cy + x), (cx - y, cy + x),
                (cx + y, cy - x), (cx - y, cy - x),
            ];

            for (px, py) in points {
                if px >= 0 && py >= 0 {
                    canvas.set_pixel(px as u32, py as u32, r, g, b, a);
                }
            }
        };

        while x >= y {
            plot(self, cx as i32, cy as i32, x, y);
            y += 1;

            if p <= 0 {
                p += 2 * y + 1;
            } else {
                x -= 1;
                p += 2 * (y - x) + 1;
            }
        }
    }

    /// Draw filled circle
    fn draw_filled_circle(&mut self, cx: u32, cy: u32, radius: u32, r: u8, g: u8, b: u8, a: u8) {
        let r_sq = (radius * radius) as i32;
        let cx_i = cx as i32;
        let cy_i = cy as i32;
        let radius_i = radius as i32;

        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                if dx * dx + dy * dy <= r_sq {
                    let px = cx_i + dx;
                    let py = cy_i + dy;

                    if px >= 0 && py >= 0 {
                        self.set_pixel(px as u32, py as u32, r, g, b, a);
                    }
                }
            }
        }
    }

    /// Draw line using Bresenham's algorithm
    fn draw_line(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, r: u8, g: u8, b: u8, a: u8) {
        let (mut x, mut y) = (x1 as i32, y1 as i32);
        let (x2, y2) = (x2 as i32, y2 as i32);

        let dx = (x2 - x).abs();
        let dy = -(y2 - y).abs();
        let sx = if x < x2 { 1 } else { -1 };
        let sy = if y < y2 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as u32, y as u32, r, g, b, a);
            }

            if x == x2 && y == y2 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Get pixel buffer
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Get alpha buffer
    pub fn alpha(&self) -> &[f32] {
        &self.alpha
    }

    /// Get canvas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Canvas layer logic - executes draw operations
#[derive(Clone)]
pub struct CanvasLogic {
    canvas: Canvas,
    /// User-provided update function
    update_fn: fn(&Canvas, f32, &dyn Controller) -> Canvas,
}

impl CanvasLogic {
    /// Create new canvas logic with update function
    pub fn new(
        width: u32,
        height: u32,
        update_fn: fn(&Canvas, f32, &dyn Controller) -> Canvas,
    ) -> Self {
        Self {
            canvas: Canvas::new(width, height),
            update_fn,
        }
    }

    /// Get canvas reference
    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }
}

impl LayerLogic for CanvasLogic {
    fn update(&self, delta: f32, controller: &dyn Controller) -> Self {
        let new_canvas = (self.update_fn)(&self.canvas, delta, controller);
        let executed = new_canvas.execute_ops();

        Self {
            canvas: executed,
            update_fn: self.update_fn,
        }
    }

    fn render(&self, _mask: &[bool], _context: &DisplayContext) -> LayerOutput {
        LayerOutput::with_alpha(
            self.canvas.pixels.clone(),
            self.canvas.alpha.clone(),
        )
    }
}

/// Builder for canvas layer
pub struct CanvasLayerBuilder {
    width: u32,
    height: u32,
    update_fn: fn(&Canvas, f32, &dyn Controller) -> Canvas,
    target_fps: f32,
    priority: i32,
}

impl CanvasLayerBuilder {
    /// Create new builder with dimensions and update function
    pub fn new(
        width: u32,
        height: u32,
        update_fn: fn(&Canvas, f32, &dyn Controller) -> Canvas,
    ) -> Self {
        Self {
            width,
            height,
            update_fn,
            target_fps: 60.0,
            priority: 0,
        }
    }

    /// Set target FPS
    pub fn fps(mut self, fps: f32) -> Self {
        self.target_fps = fps;
        self
    }

    /// Set layer priority
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Build the layer
    pub fn build(self) -> Box<dyn Layer> {
        let logic = CanvasLogic::new(self.width, self.height, self.update_fn);
        Box::new(TimedLayer::new(logic, self.target_fps, self.priority))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_creation() {
        let canvas = Canvas::new(100, 100);
        assert_eq!(canvas.dimensions(), (100, 100));
        assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
        assert_eq!(canvas.alpha().len(), 100 * 100);
    }

    #[test]
    fn canvas_clear() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::Clear(255, 0, 0, 255))
            .execute_ops();

        // Check first pixel
        assert_eq!(&canvas.pixels()[0..4], &[255, 0, 0, 255]);
        // Check last pixel
        let last_idx = 10 * 10 * 4 - 4;
        assert_eq!(&canvas.pixels()[last_idx..last_idx + 4], &[255, 0, 0, 255]);
        // Check alpha
        assert_eq!(canvas.alpha()[0], 1.0);
        assert_eq!(canvas.alpha()[99], 1.0);
    }

    #[test]
    fn canvas_set_pixel() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::Pixel { x: 5, y: 5, r: 100, g: 150, b: 200, a: 128 })
            .execute_ops();

        let idx = (5 * 10 + 5) * 4;
        assert_eq!(&canvas.pixels()[idx..idx + 4], &[100, 150, 200, 128]);
        assert!((canvas.alpha()[5 * 10 + 5] - 128.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn canvas_hline() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::HLine { x: 2, y: 5, length: 5, r: 255, g: 0, b: 0, a: 255 })
            .execute_ops();

        for x in 2..7 {
            let idx = (5 * 10 + x) * 4;
            assert_eq!(&canvas.pixels()[idx..idx + 4], &[255, 0, 0, 255]);
        }
    }

    #[test]
    fn canvas_vline() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::VLine { x: 5, y: 2, length: 5, r: 0, g: 255, b: 0, a: 255 })
            .execute_ops();

        for y in 2..7 {
            let idx = (y * 10 + 5) * 4;
            assert_eq!(&canvas.pixels()[idx..idx + 4], &[0, 255, 0, 255]);
        }
    }

    #[test]
    fn canvas_rect() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::Rect { x: 2, y: 2, width: 4, height: 3, r: 50, g: 100, b: 150, a: 200 })
            .execute_ops();

        // Check corners
        let top_left = (2 * 10 + 2) * 4;
        assert_eq!(&canvas.pixels()[top_left..top_left + 4], &[50, 100, 150, 200]);

        let bottom_right = (4 * 10 + 5) * 4;
        assert_eq!(&canvas.pixels()[bottom_right..bottom_right + 4], &[50, 100, 150, 200]);
    }

    #[test]
    fn canvas_circle() {
        let canvas = Canvas::new(50, 50)
            .draw(DrawOp::Circle { cx: 25, cy: 25, radius: 10, r: 255, g: 255, b: 255, a: 255 })
            .execute_ops();

        // Check that top point is drawn
        let top_idx = (15 * 50 + 25) * 4;
        assert_eq!(&canvas.pixels()[top_idx..top_idx + 4], &[255, 255, 255, 255]);
    }

    #[test]
    fn canvas_filled_circle() {
        let canvas = Canvas::new(50, 50)
            .draw(DrawOp::FilledCircle { cx: 25, cy: 25, radius: 5, r: 100, g: 100, b: 100, a: 255 })
            .execute_ops();

        // Check center
        let center_idx = (25 * 50 + 25) * 4;
        assert_eq!(&canvas.pixels()[center_idx..center_idx + 4], &[100, 100, 100, 255]);

        // Check a point inside radius
        let inside_idx = (23 * 50 + 25) * 4;
        assert_eq!(&canvas.pixels()[inside_idx..inside_idx + 4], &[100, 100, 100, 255]);
    }

    #[test]
    fn canvas_line() {
        let canvas = Canvas::new(50, 50)
            .draw(DrawOp::Line { x1: 10, y1: 10, x2: 20, y2: 20, r: 128, g: 128, b: 128, a: 255 })
            .execute_ops();

        // Check start point
        let start_idx = (10 * 50 + 10) * 4;
        assert_eq!(&canvas.pixels()[start_idx..start_idx + 4], &[128, 128, 128, 255]);

        // Check end point
        let end_idx = (20 * 50 + 20) * 4;
        assert_eq!(&canvas.pixels()[end_idx..end_idx + 4], &[128, 128, 128, 255]);
    }

    #[test]
    fn canvas_multiple_ops() {
        let canvas = Canvas::new(50, 50)
            .draw(DrawOp::Clear(0, 0, 0, 255))
            .draw(DrawOp::Rect { x: 10, y: 10, width: 30, height: 30, r: 255, g: 0, b: 0, a: 255 })
            .draw(DrawOp::FilledCircle { cx: 25, cy: 25, radius: 10, r: 0, g: 255, b: 0, a: 255 })
            .execute_ops();

        // Check background (should be black)
        let bg_idx = 0;
        assert_eq!(&canvas.pixels()[bg_idx..bg_idx + 4], &[0, 0, 0, 255]);

        // Check rectangle corner (red, but might be overwritten by circle)
        let rect_idx = (10 * 50 + 10) * 4;
        assert_eq!(&canvas.pixels()[rect_idx..rect_idx + 4], &[255, 0, 0, 255]);

        // Check circle center (green)
        let center_idx = (25 * 50 + 25) * 4;
        assert_eq!(&canvas.pixels()[center_idx..center_idx + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn canvas_bounds_checking() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::Pixel { x: 100, y: 100, r: 255, g: 0, b: 0, a: 255 })
            .execute_ops();

        // Should not crash - bounds checked
        assert_eq!(canvas.pixels().len(), 10 * 10 * 4);
    }

    #[test]
    fn canvas_alpha_transparency() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::Clear(255, 255, 255, 0))
            .execute_ops();

        // All pixels should be transparent
        for alpha in canvas.alpha() {
            assert_eq!(*alpha, 0.0);
        }
    }

    #[test]
    fn canvas_layer_builder() {
        fn update_canvas(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
            Canvas::new(100, 100)
        }

        let layer = CanvasLayerBuilder::new(100, 100, update_canvas)
            .fps(30.0)
            .priority(5)
            .build();

        assert!((layer.target_fps() - 30.0).abs() < 0.01);
        assert_eq!(layer.priority(), 5);
    }

    // ========================================================================
    // Comprehensive Integration Tests
    // ========================================================================

    use crate::core::controller::Button;
    use crate::core::display_context::DisplayContext;
    use crate::core::layer::{LayerStack, LayerLogic};

    /// Mock controller for testing - no keys pressed
    struct MockController;

    impl Controller for MockController {
        fn is_down(&self, _button: Button) -> bool {
            false
        }

        fn get_down_keys(&self) -> &[Button] {
            &[]
        }
    }

    /// Test controller with configurable key states
    struct TestController {
        down_keys: Vec<Button>,
    }

    impl Controller for TestController {
        fn is_down(&self, button: Button) -> bool {
            self.down_keys.contains(&button)
        }

        fn get_down_keys(&self) -> &[Button] {
            &self.down_keys
        }
    }

    // ====================================================================
    // Canvas Core Functionality Tests
    // ====================================================================

    #[test]
    fn test_canvas_new() {
        let canvas = Canvas::new(640, 480);
        assert_eq!(canvas.dimensions(), (640, 480));
        assert_eq!(canvas.pixels().len(), 640 * 480 * 4);
        assert_eq!(canvas.alpha().len(), 640 * 480);
    }

    #[test]
    fn test_canvas_clear_opaque() {
        let executed = Canvas::new(100, 100)
            .draw(DrawOp::Clear(255, 128, 64, 255))
            .execute_ops();

        let pixels = executed.pixels();
        let alpha = executed.alpha();

        assert_eq!(&pixels[0..4], &[255, 128, 64, 255]);
        assert_eq!(alpha[0], 1.0);

        let mid_idx = (50 * 100 + 50) * 4;
        assert_eq!(&pixels[mid_idx..mid_idx + 4], &[255, 128, 64, 255]);

        let last_idx = (100 * 100 - 1) * 4;
        assert_eq!(&pixels[last_idx..last_idx + 4], &[255, 128, 64, 255]);
    }

    #[test]
    fn test_canvas_clear_transparent() {
        let canvas = Canvas::new(50, 50)
            .draw(DrawOp::Clear(100, 100, 100, 0))
            .execute_ops();

        let alpha = canvas.alpha();

        for a in alpha {
            assert_eq!(*a, 0.0);
        }
    }

    #[test]
    fn test_canvas_clear_semi_transparent() {
        let canvas = Canvas::new(50, 50)
            .draw(DrawOp::Clear(200, 150, 100, 128))
            .execute_ops();

        let alpha = canvas.alpha();
        let expected_alpha = 128.0 / 255.0;

        for a in alpha {
            assert!((*a - expected_alpha).abs() < 0.01);
        }
    }

    #[test]
    fn test_canvas_pixel_in_bounds() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::Pixel { x: 50, y: 50, r: 255, g: 0, b: 128, a: 200 })
            .execute_ops();

        let idx = (50 * 100 + 50) * 4;
        let pixels = canvas.pixels();

        assert_eq!(&pixels[idx..idx + 4], &[255, 0, 128, 200]);
        assert!((canvas.alpha()[50 * 100 + 50] - 200.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_canvas_pixel_out_of_bounds() {
        let canvas = Canvas::new(10, 10)
            .draw(DrawOp::Pixel { x: 100, y: 100, r: 255, g: 0, b: 0, a: 255 })
            .execute_ops();

        assert_eq!(canvas.pixels().len(), 10 * 10 * 4);
    }

    #[test]
    fn test_canvas_pixel_edge_cases() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::Pixel { x: 0, y: 0, r: 255, g: 0, b: 0, a: 255 })
            .draw(DrawOp::Pixel { x: 99, y: 99, r: 0, g: 255, b: 0, a: 255 })
            .draw(DrawOp::Pixel { x: 0, y: 99, r: 0, g: 0, b: 255, a: 255 })
            .draw(DrawOp::Pixel { x: 99, y: 0, r: 255, g: 255, b: 0, a: 255 })
            .execute_ops();

        let pixels = canvas.pixels();

        assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);

        let br_idx = (99 * 100 + 99) * 4;
        assert_eq!(&pixels[br_idx..br_idx + 4], &[0, 255, 0, 255]);

        let bl_idx = (99 * 100 + 0) * 4;
        assert_eq!(&pixels[bl_idx..bl_idx + 4], &[0, 0, 255, 255]);

        let tr_idx = (0 * 100 + 99) * 4;
        assert_eq!(&pixels[tr_idx..tr_idx + 4], &[255, 255, 0, 255]);
    }

    // ====================================================================
    // Line Drawing Tests
    // ====================================================================

    #[test]
    fn test_canvas_hline_basic() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::HLine { x: 10, y: 50, length: 20, r: 255, g: 0, b: 0, a: 255 })
            .execute_ops();

        let pixels = canvas.pixels();

        for x in 10..30 {
            let idx = (50 * 100 + x) * 4;
            assert_eq!(&pixels[idx..idx + 4], &[255, 0, 0, 255], "Failed at x={}", x);
        }
    }

    #[test]
    fn test_canvas_hline_zero_length() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::HLine { x: 10, y: 50, length: 0, r: 255, g: 0, b: 0, a: 255 })
            .execute_ops();

        assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
    }

    #[test]
    fn test_canvas_vline_basic() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::VLine { x: 50, y: 10, length: 20, r: 0, g: 255, b: 0, a: 255 })
            .execute_ops();

        let pixels = canvas.pixels();

        for y in 10..30 {
            let idx = (y * 100 + 50) * 4;
            assert_eq!(&pixels[idx..idx + 4], &[0, 255, 0, 255], "Failed at y={}", y);
        }
    }

    #[test]
    fn test_canvas_vline_zero_length() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::VLine { x: 50, y: 10, length: 0, r: 0, g: 255, b: 0, a: 255 })
            .execute_ops();

        assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
    }

    #[test]
    fn test_canvas_line_horizontal() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::Line { x1: 10, y1: 50, x2: 30, y2: 50, r: 128, g: 128, b: 128, a: 255 })
            .execute_ops();

        let pixels = canvas.pixels();

        let start_idx = (50 * 100 + 10) * 4;
        assert_eq!(&pixels[start_idx..start_idx + 4], &[128, 128, 128, 255]);

        let end_idx = (50 * 100 + 30) * 4;
        assert_eq!(&pixels[end_idx..end_idx + 4], &[128, 128, 128, 255]);
    }

    #[test]
    fn test_canvas_line_vertical() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::Line { x1: 50, y1: 10, x2: 50, y2: 30, r: 128, g: 128, b: 128, a: 255 })
            .execute_ops();

        let pixels = canvas.pixels();

        let start_idx = (10 * 100 + 50) * 4;
        assert_eq!(&pixels[start_idx..start_idx + 4], &[128, 128, 128, 255]);

        let end_idx = (30 * 100 + 50) * 4;
        assert_eq!(&pixels[end_idx..end_idx + 4], &[128, 128, 128, 255]);
    }

    #[test]
    fn test_canvas_line_diagonal() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::Line { x1: 10, y1: 10, x2: 30, y2: 30, r: 255, g: 255, b: 0, a: 255 })
            .execute_ops();

        let pixels = canvas.pixels();

        let start_idx = (10 * 100 + 10) * 4;
        assert_eq!(&pixels[start_idx..start_idx + 4], &[255, 255, 0, 255]);

        let end_idx = (30 * 100 + 30) * 4;
        assert_eq!(&pixels[end_idx..end_idx + 4], &[255, 255, 0, 255]);

        let mid_idx = (20 * 100 + 20) * 4;
        assert_eq!(&pixels[mid_idx..mid_idx + 4], &[255, 255, 0, 255]);
    }

    #[test]
    fn test_canvas_line_single_point() {
        let canvas = Canvas::new(100, 100)
            .draw(DrawOp::Line { x1: 50, y1: 50, x2: 50, y2: 50, r: 200, g: 100, b: 50, a: 255 })
            .execute_ops();

        let pixels = canvas.pixels();
        let idx = (50 * 100 + 50) * 4;
        assert_eq!(&pixels[idx..idx + 4], &[200, 100, 50, 255]);
    }


// ============================================================================
// Rectangle Tests
// ============================================================================

#[test]
fn test_canvas_rect_basic() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Rect { x: 10, y: 10, width: 20, height: 15, r: 100, g: 150, b: 200, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Check corners
    let tl_idx = (10 * 100 + 10) * 4;
    assert_eq!(&pixels[tl_idx..tl_idx + 4], &[100, 150, 200, 255]);

    let tr_idx = (10 * 100 + 29) * 4;
    assert_eq!(&pixels[tr_idx..tr_idx + 4], &[100, 150, 200, 255]);

    let bl_idx = (24 * 100 + 10) * 4;
    assert_eq!(&pixels[bl_idx..bl_idx + 4], &[100, 150, 200, 255]);

    let br_idx = (24 * 100 + 29) * 4;
    assert_eq!(&pixels[br_idx..br_idx + 4], &[100, 150, 200, 255]);

    // Check center
    let center_idx = (17 * 100 + 20) * 4;
    assert_eq!(&pixels[center_idx..center_idx + 4], &[100, 150, 200, 255]);
}

#[test]
fn test_canvas_rect_zero_size() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Rect { x: 10, y: 10, width: 0, height: 0, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
}

#[test]
fn test_canvas_rect_single_pixel() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Rect { x: 50, y: 50, width: 1, height: 1, r: 255, g: 128, b: 64, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let idx = (50 * 100 + 50) * 4;
    assert_eq!(&pixels[idx..idx + 4], &[255, 128, 64, 255]);
}

#[test]
fn test_canvas_rect_partial_out_of_bounds() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Rect { x: 90, y: 90, width: 20, height: 20, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Should draw the visible portion
    let idx = (90 * 100 + 90) * 4;
    assert_eq!(&pixels[idx..idx + 4], &[255, 0, 0, 255]);
}

// ============================================================================
// Circle Tests
// ============================================================================

#[test]
fn test_canvas_circle_basic() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Circle { cx: 50, cy: 50, radius: 20, r: 255, g: 255, b: 255, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Check top point (approximately)
    let top_idx = (30 * 100 + 50) * 4;
    assert_eq!(&pixels[top_idx..top_idx + 4], &[255, 255, 255, 255]);

    // Check right point (approximately)
    let right_idx = (50 * 100 + 70) * 4;
    assert_eq!(&pixels[right_idx..right_idx + 4], &[255, 255, 255, 255]);
}

#[test]
fn test_canvas_circle_zero_radius() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Circle { cx: 50, cy: 50, radius: 0, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    // Should handle gracefully
    assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
}

#[test]
fn test_canvas_filled_circle_basic() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::FilledCircle { cx: 50, cy: 50, radius: 10, r: 100, g: 100, b: 100, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Check center
    let center_idx = (50 * 100 + 50) * 4;
    assert_eq!(&pixels[center_idx..center_idx + 4], &[100, 100, 100, 255]);

    // Check point inside
    let inside_idx = (48 * 100 + 50) * 4;
    assert_eq!(&pixels[inside_idx..inside_idx + 4], &[100, 100, 100, 255]);

    // Check point on edge
    let edge_idx = (40 * 100 + 50) * 4;
    assert_eq!(&pixels[edge_idx..edge_idx + 4], &[100, 100, 100, 255]);
}

#[test]
fn test_canvas_filled_circle_coverage() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Clear(0, 0, 0, 255))
        .draw(DrawOp::FilledCircle { cx: 50, cy: 50, radius: 5, r: 255, g: 255, b: 255, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let mut filled_count = 0;

    for y in 0..100 {
        for x in 0..100 {
            let idx = (y * 100 + x) * 4;
            if pixels[idx] == 255 {
                filled_count += 1;
            }
        }
    }

    // Should be roughly pi * r^2 = pi * 25 ≈ 78
    assert!(filled_count > 70 && filled_count < 85);
}

// ============================================================================
// Composite Operations Tests
// ============================================================================

#[test]
fn test_canvas_multiple_operations_sequential() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Clear(255, 255, 255, 255))
        .draw(DrawOp::Rect { x: 20, y: 20, width: 60, height: 60, r: 0, g: 0, b: 255, a: 255 })
        .draw(DrawOp::FilledCircle { cx: 50, cy: 50, radius: 20, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Background (white)
    let bg_idx = (0 * 100 + 0) * 4;
    assert_eq!(&pixels[bg_idx..bg_idx + 4], &[255, 255, 255, 255]);

    // Rectangle area (blue)
    let rect_idx = (25 * 100 + 25) * 4;
    assert_eq!(&pixels[rect_idx..rect_idx + 4], &[0, 0, 255, 255]);

    // Circle center (red - overwrites blue)
    let circle_idx = (50 * 100 + 50) * 4;
    assert_eq!(&pixels[circle_idx..circle_idx + 4], &[255, 0, 0, 255]);
}

#[test]
fn test_canvas_layered_transparency() {
    let canvas = Canvas::new(50, 50)
        .draw(DrawOp::Clear(0, 0, 0, 255))
        .draw(DrawOp::Rect { x: 10, y: 10, width: 30, height: 30, r: 255, g: 0, b: 0, a: 128 })
        .execute_ops();

    let alpha = canvas.alpha();

    // Inside rectangle - semi-transparent
    let inside_alpha = alpha[15 * 50 + 15];
    assert!((inside_alpha - 128.0 / 255.0).abs() < 0.01);

    // Outside rectangle - opaque
    let outside_alpha = alpha[0];
    assert_eq!(outside_alpha, 1.0);
}

#[test]
fn test_canvas_complex_scene() {
    let canvas = Canvas::new(200, 200)
        .draw(DrawOp::Clear(50, 50, 50, 255))
        .draw(DrawOp::Rect { x: 20, y: 20, width: 160, height: 160, r: 100, g: 100, b: 200, a: 255 })
        .draw(DrawOp::FilledCircle { cx: 100, cy: 100, radius: 50, r: 255, g: 200, b: 0, a: 255 })
        .draw(DrawOp::Circle { cx: 100, cy: 100, radius: 70, r: 255, g: 255, b: 255, a: 255 })
        .draw(DrawOp::Line { x1: 50, y1: 50, x2: 150, y2: 150, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Verify different regions have expected colors
    assert_eq!(canvas.pixels().len(), 200 * 200 * 4);

    // Center should be red from diagonal line (drawn last, overwrites circle)
    let center_idx = (100 * 200 + 100) * 4;
    assert_eq!(&pixels[center_idx..center_idx + 4], &[255, 0, 0, 255]);

    // Check a point in the yellow circle that isn't on the line
    let circle_point_idx = (95 * 200 + 100) * 4;
    assert_eq!(&pixels[circle_point_idx..circle_point_idx + 4], &[255, 200, 0, 255]);
}

// ============================================================================
// Layer Logic Tests
// ============================================================================

#[test]
fn test_canvas_logic_creation() {
    fn update_fn(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(100, 100)
    }

    let logic = CanvasLogic::new(100, 100, update_fn);
    assert_eq!(logic.canvas().dimensions(), (100, 100));
}

#[test]
fn test_canvas_logic_update() {
    fn update_fn(canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(canvas.dimensions().0, canvas.dimensions().1)
            .draw(DrawOp::Clear(255, 0, 0, 255))
    }

    let logic = CanvasLogic::new(50, 50, update_fn);
    let controller = MockController;

    let updated = logic.update(0.016, &controller);
    let rendered = updated.render(&vec![true; 50 * 50], &DisplayContext::new(50, 50));

    // Should have red background
    assert_eq!(&rendered.pixels[0..4], &[255, 0, 0, 255]);
}

#[test]
fn test_canvas_logic_render_with_alpha() {
    fn update_fn(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(10, 10)
            .draw(DrawOp::Clear(100, 100, 100, 128))
    }

    let logic = CanvasLogic::new(10, 10, update_fn);
    let controller = MockController;

    let updated = logic.update(0.016, &controller);
    let context = DisplayContext::new(10, 10);
    let output = updated.render(&vec![true; 10 * 10], &context);

    // Check alpha channel
    let alpha = output.alpha.unwrap();
    assert!((alpha[0] - 128.0 / 255.0).abs() < 0.01);
}

// ============================================================================
// Layer Builder Tests
// ============================================================================

#[test]
fn test_canvas_layer_builder_default() {
    fn update_fn(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(100, 100)
    }

    let layer = CanvasLayerBuilder::new(100, 100, update_fn).build();

    assert!((layer.target_fps() - 60.0).abs() < 0.01);
    assert_eq!(layer.priority(), 0);
}

#[test]
fn test_canvas_layer_builder_custom() {
    fn update_fn(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(200, 150)
    }

    let layer = CanvasLayerBuilder::new(200, 150, update_fn)
        .fps(30.0)
        .priority(5)
        .build();

    assert!((layer.target_fps() - 30.0).abs() < 0.01);
    assert_eq!(layer.priority(), 5);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_canvas_layer_in_stack() {
    fn update_fn(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(100, 100)
            .draw(DrawOp::Clear(255, 0, 0, 255))
    }

    let layer = CanvasLayerBuilder::new(100, 100, update_fn)
        .fps(60.0)
        .priority(0)
        .build();

    let stack = LayerStack::new().with_layer(layer);
    let controller = MockController;

    // First update - will trigger since delta >= 1/60
    let delta = 0.017;
    let updated_stack = stack.update(delta, &controller);

    let context = DisplayContext::new(100, 100);
    let mask = vec![true; 100 * 100];

    let outputs: Vec<_> = updated_stack.render(&mask, &context).collect();

    assert_eq!(outputs.len(), 1);
    assert_eq!(&outputs[0].pixels[0..4], &[255, 0, 0, 255]);
}

#[test]
fn test_multiple_canvas_layers() {
    fn background_update(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(100, 100)
            .draw(DrawOp::Clear(0, 0, 255, 255))
    }

    fn foreground_update(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(100, 100)
            .draw(DrawOp::FilledCircle { cx: 50, cy: 50, radius: 20, r: 255, g: 0, b: 0, a: 255 })
    }

    let bg_layer = CanvasLayerBuilder::new(100, 100, background_update)
        .priority(0)
        .build();

    let fg_layer = CanvasLayerBuilder::new(100, 100, foreground_update)
        .priority(10)
        .build();

    let stack = LayerStack::new()
        .with_layer(bg_layer)
        .with_layer(fg_layer);

    let delta = 0.017;
    let controller = MockController;
    let updated_stack = stack.update(delta, &controller);

    let context = DisplayContext::new(100, 100);
    let mask = vec![true; 100 * 100];

    let outputs: Vec<_> = updated_stack.render(&mask, &context).collect();
    assert_eq!(outputs.len(), 2);
}

#[test]
fn test_canvas_layer_timing() {
    static mut UPDATE_COUNT: u32 = 0;

    fn counting_update(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        unsafe {
            UPDATE_COUNT += 1;
        }
        Canvas::new(10, 10)
    }

    let layer = CanvasLayerBuilder::new(10, 10, counting_update)
        .fps(30.0)
        .build();

    let controller = MockController;

    // First update - small delta, should not trigger
    let layer = layer.update(0.01, &controller);

    // Second update - still accumulating (total 0.026s < 0.033s)
    let layer = layer.update(0.016, &controller);

    // Third update - now trigger (total 0.044s >= 0.033s)
    let _layer = layer.update(0.018, &controller);

    unsafe {
        assert!(UPDATE_COUNT >= 1);
    }
}

#[test]
fn test_canvas_functional_composition() {
    let canvas = Canvas::new(50, 50)
        .draw(DrawOp::Clear(0, 0, 0, 255))
        .draw(DrawOp::Rect { x: 10, y: 10, width: 30, height: 30, r: 255, g: 0, b: 0, a: 255 })
        .draw(DrawOp::Circle { cx: 25, cy: 25, radius: 10, r: 0, g: 255, b: 0, a: 255 });

    // Operations should be queued, not executed
    assert_eq!(canvas.dimensions(), (50, 50));

    let executed = canvas.execute_ops();
    let pixels = executed.pixels();

    // Background
    let bg_idx = 0;
    assert_eq!(&pixels[bg_idx..bg_idx + 4], &[0, 0, 0, 255]);

    // Rectangle area
    let rect_idx = (15 * 50 + 15) * 4;
    assert_eq!(&pixels[rect_idx..rect_idx + 4], &[255, 0, 0, 255]);
}

#[test]
fn test_canvas_drawop_equality() {
    let op1 = DrawOp::Clear(255, 0, 0, 255);
    let op2 = DrawOp::Clear(255, 0, 0, 255);
    let op3 = DrawOp::Clear(0, 255, 0, 255);

    assert_eq!(op1, op2);
    assert_ne!(op1, op3);
}

#[test]
fn test_canvas_stress_many_operations() {
    let mut canvas = Canvas::new(500, 500)
        .draw(DrawOp::Clear(0, 0, 0, 255));

    // Add 100 random draw operations
    for i in 0..100 {
        let x = (i * 7) % 500;
        let y = (i * 11) % 500;
        canvas = canvas.draw(DrawOp::Pixel { x, y, r: 255, g: 255, b: 255, a: 255 });
    }

    let executed = canvas.execute_ops();
    assert_eq!(executed.pixels().len(), 500 * 500 * 4);
}

#[test]
fn test_canvas_performance_large() {
    let canvas = Canvas::new(1920, 1080)
        .draw(DrawOp::Clear(128, 128, 128, 255))
        .execute_ops();

    assert_eq!(canvas.pixels().len(), 1920 * 1080 * 4);
    assert_eq!(canvas.alpha().len(), 1920 * 1080);
}

// ============================================================================
// Bresenham Line Algorithm - All Octants
// ============================================================================

#[test]
fn test_line_octant_0() {
    // Octant 0: dx > dy, moving right and slightly up
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 10, y1: 50, x2: 40, y2: 40, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (50 * 100 + 10) * 4;
    let end = (40 * 100 + 40) * 4;

    assert_eq!(&pixels[start..start + 4], &[255, 0, 0, 255]);
    assert_eq!(&pixels[end..end + 4], &[255, 0, 0, 255]);
}

#[test]
fn test_line_octant_1() {
    // Octant 1: dy > dx, moving right and up
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 50, y1: 80, x2: 60, y2: 50, r: 0, g: 255, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (80 * 100 + 50) * 4;
    let end = (50 * 100 + 60) * 4;

    assert_eq!(&pixels[start..start + 4], &[0, 255, 0, 255]);
    assert_eq!(&pixels[end..end + 4], &[0, 255, 0, 255]);
}

#[test]
fn test_line_octant_2() {
    // Octant 2: dy > dx, moving left and up
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 60, y1: 80, x2: 50, y2: 50, r: 0, g: 0, b: 255, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (80 * 100 + 60) * 4;
    let end = (50 * 100 + 50) * 4;

    assert_eq!(&pixels[start..start + 4], &[0, 0, 255, 255]);
    assert_eq!(&pixels[end..end + 4], &[0, 0, 255, 255]);
}

#[test]
fn test_line_octant_3() {
    // Octant 3: dx > dy, moving left and slightly up
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 70, y1: 50, x2: 40, y2: 40, r: 255, g: 255, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (50 * 100 + 70) * 4;
    let end = (40 * 100 + 40) * 4;

    assert_eq!(&pixels[start..start + 4], &[255, 255, 0, 255]);
    assert_eq!(&pixels[end..end + 4], &[255, 255, 0, 255]);
}

#[test]
fn test_line_octant_4() {
    // Octant 4: dx > dy, moving left and slightly down
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 70, y1: 40, x2: 40, y2: 50, r: 255, g: 0, b: 255, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (40 * 100 + 70) * 4;
    let end = (50 * 100 + 40) * 4;

    assert_eq!(&pixels[start..start + 4], &[255, 0, 255, 255]);
    assert_eq!(&pixels[end..end + 4], &[255, 0, 255, 255]);
}

#[test]
fn test_line_octant_5() {
    // Octant 5: dy > dx, moving left and down
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 60, y1: 40, x2: 50, y2: 70, r: 0, g: 255, b: 255, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (40 * 100 + 60) * 4;
    let end = (70 * 100 + 50) * 4;

    assert_eq!(&pixels[start..start + 4], &[0, 255, 255, 255]);
    assert_eq!(&pixels[end..end + 4], &[0, 255, 255, 255]);
}

#[test]
fn test_line_octant_6() {
    // Octant 6: dy > dx, moving right and down
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 40, y1: 40, x2: 50, y2: 70, r: 128, g: 128, b: 128, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (40 * 100 + 40) * 4;
    let end = (70 * 100 + 50) * 4;

    assert_eq!(&pixels[start..start + 4], &[128, 128, 128, 255]);
    assert_eq!(&pixels[end..end + 4], &[128, 128, 128, 255]);
}

#[test]
fn test_line_octant_7() {
    // Octant 7: dx > dy, moving right and slightly down
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 10, y1: 40, x2: 40, y2: 50, r: 200, g: 100, b: 50, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (40 * 100 + 10) * 4;
    let end = (50 * 100 + 40) * 4;

    assert_eq!(&pixels[start..start + 4], &[200, 100, 50, 255]);
    assert_eq!(&pixels[end..end + 4], &[200, 100, 50, 255]);
}

// ============================================================================
// Negative Coordinates and Line Edge Cases
// ============================================================================

#[test]
fn test_line_with_negative_intermediate() {
    // Line that would have negative coordinates during calculation
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 5, y1: 5, x2: 0, y2: 0, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let origin = 0;
    assert_eq!(&pixels[origin..origin + 4], &[255, 0, 0, 255]);
}

#[test]
fn test_line_steep_slope() {
    // Very steep line (close to vertical)
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 50, y1: 10, x2: 51, y2: 80, r: 255, g: 128, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (10 * 100 + 50) * 4;
    let end = (80 * 100 + 51) * 4;

    assert_eq!(&pixels[start..start + 4], &[255, 128, 0, 255]);
    assert_eq!(&pixels[end..end + 4], &[255, 128, 0, 255]);
}

#[test]
fn test_line_shallow_slope() {
    // Very shallow line (close to horizontal)
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Line { x1: 10, y1: 50, x2: 80, y2: 51, r: 0, g: 128, b: 255, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();
    let start = (50 * 100 + 10) * 4;
    let end = (51 * 100 + 80) * 4;

    assert_eq!(&pixels[start..start + 4], &[0, 128, 255, 255]);
    assert_eq!(&pixels[end..end + 4], &[0, 128, 255, 255]);
}

// ============================================================================
// Controller Interaction Tests
// ============================================================================

#[test]
fn test_canvas_logic_with_controller_input() {
    fn update_with_input(canvas: &Canvas, _delta: f32, controller: &dyn Controller) -> Canvas {
        let color = if controller.is_down(Button::Space) {
            (255, 0, 0)
        } else {
            (0, 255, 0)
        };

        Canvas::new(canvas.dimensions().0, canvas.dimensions().1)
            .draw(DrawOp::Clear(color.0, color.1, color.2, 255))
    }

    let logic = CanvasLogic::new(50, 50, update_with_input);

    // Test with space key down
    let controller_space = TestController {
        down_keys: vec![Button::Space],
    };
    let updated = logic.update(0.016, &controller_space);
    let context = DisplayContext::new(50, 50);
    let output = updated.render(&vec![true; 50 * 50], &context);

    assert_eq!(&output.pixels[0..4], &[255, 0, 0, 255]); // Red

    // Test without space key
    let controller_none = TestController {
        down_keys: vec![],
    };
    let updated2 = logic.update(0.016, &controller_none);
    let output2 = updated2.render(&vec![true; 50 * 50], &context);

    assert_eq!(&output2.pixels[0..4], &[0, 255, 0, 255]); // Green
}

#[test]
fn test_canvas_logic_with_multiple_keys() {
    fn update_with_keys(_canvas: &Canvas, _delta: f32, controller: &dyn Controller) -> Canvas {
        let mut canvas = Canvas::new(100, 100)
            .draw(DrawOp::Clear(0, 0, 0, 255));

        if controller.is_down(Button::KeyW) {
            canvas = canvas.draw(DrawOp::Rect { x: 40, y: 20, width: 20, height: 20, r: 255, g: 0, b: 0, a: 255 });
        }
        if controller.is_down(Button::KeyS) {
            canvas = canvas.draw(DrawOp::Rect { x: 40, y: 60, width: 20, height: 20, r: 0, g: 255, b: 0, a: 255 });
        }
        if controller.is_down(Button::Space) {
            canvas = canvas.draw(DrawOp::FilledCircle { cx: 50, cy: 50, radius: 10, r: 255, g: 255, b: 0, a: 255 });
        }

        canvas
    }

    let logic = CanvasLogic::new(100, 100, update_with_keys);

    let controller = TestController {
        down_keys: vec![Button::KeyW, Button::Space],
    };

    let updated = logic.update(0.016, &controller);
    let context = DisplayContext::new(100, 100);
    let output = updated.render(&vec![true; 100 * 100], &context);

    // Check that both shapes are drawn
    let rect_idx = (25 * 100 + 45) * 4;
    assert_eq!(&output.pixels[rect_idx..rect_idx + 4], &[255, 0, 0, 255]);

    let circle_idx = (50 * 100 + 50) * 4;
    assert_eq!(&output.pixels[circle_idx..circle_idx + 4], &[255, 255, 0, 255]);
}

// ============================================================================
// Mask Rendering Tests
// ============================================================================

#[test]
fn test_render_with_empty_mask() {
    fn simple_update(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(10, 10)
            .draw(DrawOp::Clear(255, 0, 0, 255))
    }

    let logic = CanvasLogic::new(10, 10, simple_update);
    let controller = MockController;

    let updated = logic.update(0.016, &controller);
    let context = DisplayContext::new(10, 10);
    let mask = vec![false; 10 * 10]; // All masked out

    let output = updated.render(&mask, &context);

    // Should still render even with empty mask
    assert_eq!(output.pixels.len(), 10 * 10 * 4);
}

#[test]
fn test_render_with_partial_mask() {
    fn simple_update(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(10, 10)
            .draw(DrawOp::Clear(100, 150, 200, 255))
    }

    let logic = CanvasLogic::new(10, 10, simple_update);
    let controller = MockController;

    let updated = logic.update(0.016, &controller);
    let context = DisplayContext::new(10, 10);

    // Checkerboard pattern mask
    let mut mask = vec![false; 10 * 10];
    for i in 0..10 * 10 {
        mask[i] = i % 2 == 0;
    }

    let output = updated.render(&mask, &context);

    assert_eq!(output.pixels.len(), 10 * 10 * 4);
    assert_eq!(&output.pixels[0..4], &[100, 150, 200, 255]);
}

// ============================================================================
// Large Shape Edge Cases
// ============================================================================

#[test]
fn test_circle_very_large_radius() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Circle { cx: 50, cy: 50, radius: 1000, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    // Should not crash
    assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
}

#[test]
fn test_filled_circle_partially_offscreen() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::FilledCircle { cx: 5, cy: 5, radius: 10, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Center should be drawn
    let center = (5 * 100 + 5) * 4;
    assert_eq!(&pixels[center..center + 4], &[255, 0, 0, 255]);

    // Should not crash even though part is offscreen
    assert_eq!(pixels.len(), 100 * 100 * 4);
}

#[test]
fn test_circle_completely_offscreen() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Circle { cx: 200, cy: 200, radius: 10, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    // Should handle gracefully
    assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
}

#[test]
fn test_rectangle_completely_offscreen() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Rect { x: 200, y: 200, width: 50, height: 50, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    // Should not crash
    assert_eq!(canvas.pixels().len(), 100 * 100 * 4);
}

// ============================================================================
// Empty Operations and No-op Scenarios
// ============================================================================

#[test]
fn test_canvas_no_operations() {
    let canvas = Canvas::new(50, 50).execute_ops();

    // Should return valid canvas with black pixels
    assert_eq!(canvas.pixels().len(), 50 * 50 * 4);
    assert_eq!(canvas.alpha().len(), 50 * 50);

    // All pixels should be zero (default)
    assert_eq!(&canvas.pixels()[0..4], &[0, 0, 0, 0]);
}

#[test]
fn test_execute_ops_multiple_times() {
    let canvas = Canvas::new(20, 20)
        .draw(DrawOp::Clear(100, 100, 100, 255));

    let executed1 = canvas.execute_ops();
    let executed2 = executed1.execute_ops(); // Execute again on already executed canvas

    // Should work without adding new operations
    assert_eq!(&executed2.pixels()[0..4], &[100, 100, 100, 255]);
}

#[test]
fn test_canvas_clone_preserves_state() {
    let canvas1 = Canvas::new(30, 30)
        .draw(DrawOp::Clear(255, 0, 0, 255))
        .draw(DrawOp::Pixel { x: 15, y: 15, r: 0, g: 255, b: 0, a: 255 });

    let canvas2 = canvas1.clone();

    let executed1 = canvas1.execute_ops();
    let executed2 = canvas2.execute_ops();

    // Both should have the same result
    let idx = (15 * 30 + 15) * 4;
    assert_eq!(&executed1.pixels()[idx..idx + 4], &executed2.pixels()[idx..idx + 4]);
}

// ============================================================================
// Alpha Blending Edge Cases
// ============================================================================

#[test]
fn test_alpha_values_precision() {
    let canvas = Canvas::new(10, 10);

    // Test various alpha values
    let alphas = [0, 1, 127, 128, 254, 255];

    for &alpha in &alphas {
        let c = canvas
            .clone()
            .draw(DrawOp::Clear(100, 100, 100, alpha))
            .execute_ops();

        let expected_alpha = alpha as f32 / 255.0;
        assert!((c.alpha()[0] - expected_alpha).abs() < 0.001);
    }
}

#[test]
fn test_overlapping_shapes_alpha() {
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Clear(0, 0, 0, 255))
        .draw(DrawOp::Rect { x: 20, y: 20, width: 40, height: 40, r: 255, g: 0, b: 0, a: 128 })
        .draw(DrawOp::Rect { x: 30, y: 30, width: 40, height: 40, r: 0, g: 255, b: 0, a: 128 })
        .execute_ops();

    let pixels = canvas.pixels();
    let alpha = canvas.alpha();

    // Overlap region - second rect overwrites first
    let overlap_idx = (35 * 100 + 35) * 4;
    assert_eq!(&pixels[overlap_idx..overlap_idx + 4], &[0, 255, 0, 128]);
    assert!((alpha[35 * 100 + 35] - 128.0 / 255.0).abs() < 0.01);
}

// ============================================================================
// Dimension Edge Cases
// ============================================================================

#[test]
fn test_canvas_1x1() {
    let canvas = Canvas::new(1, 1)
        .draw(DrawOp::Pixel { x: 0, y: 0, r: 123, g: 45, b: 67, a: 255 })
        .execute_ops();

    assert_eq!(&canvas.pixels()[0..4], &[123, 45, 67, 255]);
    assert_eq!(canvas.alpha()[0], 1.0);
}

#[test]
fn test_canvas_rectangular_dimensions() {
    let canvas = Canvas::new(200, 50)
        .draw(DrawOp::HLine { x: 0, y: 25, length: 200, r: 255, g: 0, b: 0, a: 255 })
        .execute_ops();

    let pixels = canvas.pixels();

    // Check start of line
    let start = (25 * 200 + 0) * 4;
    assert_eq!(&pixels[start..start + 4], &[255, 0, 0, 255]);

    // Check end of line
    let end = (25 * 200 + 199) * 4;
    assert_eq!(&pixels[end..end + 4], &[255, 0, 0, 255]);
}

// ============================================================================
// Delta Time Accumulation Tests
// ============================================================================

#[test]
fn test_layer_timing_accumulation() {
    static mut CALL_COUNT: u32 = 0;

    fn counting_fn(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        unsafe {
            CALL_COUNT += 1;
        }
        Canvas::new(10, 10)
    }

    let layer = CanvasLayerBuilder::new(10, 10, counting_fn)
        .fps(60.0) // ~16.67ms per frame
        .build();

    let controller = MockController;

    unsafe { CALL_COUNT = 0; }

    // Multiple small updates that don't trigger
    let layer = layer.update(0.005, &controller); // 5ms
    let layer = layer.update(0.005, &controller); // 10ms total
    let layer = layer.update(0.005, &controller); // 15ms total

    // Should not have triggered yet
    unsafe {
        assert_eq!(CALL_COUNT, 0);
    }

    // Push over threshold
    let _layer = layer.update(0.005, &controller); // 20ms total > 16.67ms

    unsafe {
        assert!(CALL_COUNT >= 1);
    }
}

#[test]
fn test_layer_timing_different_fps() {
    let layer_60fps = CanvasLayerBuilder::new(10, 10, |_c, _d, _ctrl| Canvas::new(10, 10))
        .fps(60.0)
        .build();

    let layer_30fps = CanvasLayerBuilder::new(10, 10, |_c, _d, _ctrl| Canvas::new(10, 10))
        .fps(30.0)
        .build();

    assert!((layer_60fps.target_fps() - 60.0).abs() < 0.01);
    assert!((layer_30fps.target_fps() - 30.0).abs() < 0.01);
}
}
