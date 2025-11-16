use ray_tracer::core::{
    Button, Canvas, CanvasLayerBuilder, CanvasLogic, Controller, DisplayContext, DrawOp,
    LayerLogic, LayerStack,
};

/// Mock controller for testing
struct MockController;

impl Controller for MockController {
    fn is_down(&self, _button: Button) -> bool {
        false
    }

    fn get_down_keys(&self) -> &[Button] {
        &[]
    }
}

// ============================================================================
// Canvas Core Functionality Tests
// ============================================================================

#[test]
fn test_canvas_new() {
    let canvas = Canvas::new(640, 480);
    assert_eq!(canvas.dimensions(), (640, 480));
    assert_eq!(canvas.pixels().len(), 640 * 480 * 4);
    assert_eq!(canvas.alpha().len(), 640 * 480);
}

#[test]
fn test_canvas_clear_opaque() {
    // Execute without clearing first
    let executed = Canvas::new(100, 100)
        .draw(DrawOp::Clear(255, 128, 64, 255))
        .execute_ops();

    let pixels = executed.pixels();
    let alpha = executed.alpha();

    // Verify first pixel
    assert_eq!(&pixels[0..4], &[255, 128, 64, 255]);
    assert_eq!(alpha[0], 1.0);

    // Verify middle pixel
    let mid_idx = (50 * 100 + 50) * 4;
    assert_eq!(&pixels[mid_idx..mid_idx + 4], &[255, 128, 64, 255]);

    // Verify last pixel
    let last_idx = (100 * 100 - 1) * 4;
    assert_eq!(&pixels[last_idx..last_idx + 4], &[255, 128, 64, 255]);
}

#[test]
fn test_canvas_clear_transparent() {
    let canvas = Canvas::new(50, 50)
        .draw(DrawOp::Clear(100, 100, 100, 0))
        .execute_ops();

    let alpha = canvas.alpha();

    // All pixels should be fully transparent
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

    // Should not panic - bounds checked
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

    // Top-left
    assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);

    // Bottom-right
    let br_idx = (99 * 100 + 99) * 4;
    assert_eq!(&pixels[br_idx..br_idx + 4], &[0, 255, 0, 255]);

    // Bottom-left
    let bl_idx = (99 * 100 + 0) * 4;
    assert_eq!(&pixels[bl_idx..bl_idx + 4], &[0, 0, 255, 255]);

    // Top-right
    let tr_idx = (0 * 100 + 99) * 4;
    assert_eq!(&pixels[tr_idx..tr_idx + 4], &[255, 255, 0, 255]);
}

// ============================================================================
// Line Drawing Tests
// ============================================================================

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

    // Should not crash
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

    // Check start and end
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

    // Check start
    let start_idx = (10 * 100 + 10) * 4;
    assert_eq!(&pixels[start_idx..start_idx + 4], &[255, 255, 0, 255]);

    // Check end
    let end_idx = (30 * 100 + 30) * 4;
    assert_eq!(&pixels[end_idx..end_idx + 4], &[255, 255, 0, 255]);

    // Check middle point (approximate)
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
