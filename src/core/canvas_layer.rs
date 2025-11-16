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

        assert_eq!(layer.target_fps(), 30.0);
        assert_eq!(layer.priority(), 5);
    }
}
