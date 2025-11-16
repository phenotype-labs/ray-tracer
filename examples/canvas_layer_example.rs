/// Example demonstrating the 2D canvas layer system
///
/// This example shows how to create and use canvas layers with the display system,
/// including drawing primitives, compositing multiple layers, and animation.

use ray_tracer::core::{
    Button, Canvas, CanvasLayerBuilder, Controller, DisplayContext, DrawOp, Frame, LayerStack,
};

/// Mock controller for demonstration
struct DemoController;

impl Controller for DemoController {
    fn is_down(&self, _button: Button) -> bool {
        false
    }

    fn get_down_keys(&self) -> &[Button] {
        &[]
    }
}

fn main() {
    println!("2D Canvas Layer Examples\n");

    // ========================================================================
    // Example 1: Basic canvas drawing
    // ========================================================================
    println!("Example 1: Basic Canvas Drawing");
    println!("--------------------------------");

    let canvas = Canvas::new(200, 200)
        .draw(DrawOp::Clear(255, 255, 255, 255))
        .draw(DrawOp::Rect {
            x: 50,
            y: 50,
            width: 100,
            height: 100,
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        })
        .draw(DrawOp::FilledCircle {
            cx: 100,
            cy: 100,
            radius: 30,
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        })
        .execute_ops();

    println!("Created canvas: {:?}", canvas.dimensions());
    println!("Pixel buffer size: {} bytes\n", canvas.pixels().len());

    // ========================================================================
    // Example 2: Animated canvas layer
    // ========================================================================
    println!("Example 2: Animated Canvas Layer");
    println!("---------------------------------");

    fn bouncing_ball(_canvas: &Canvas, delta: f32, _controller: &dyn Controller) -> Canvas {
        // Simple bouncing ball animation (position based on time)
        static mut TIME: f32 = 0.0;
        unsafe {
            TIME += delta;
        }

        let t = unsafe { TIME };
        let x = 100.0 + (t * 2.0).sin() * 50.0;
        let y = 100.0 + (t * 3.0).cos() * 50.0;

        Canvas::new(200, 200)
            .draw(DrawOp::Clear(0, 0, 0, 255))
            .draw(DrawOp::FilledCircle {
                cx: x as u32,
                cy: y as u32,
                radius: 15,
                r: 255,
                g: 255,
                b: 0,
                a: 255,
            })
    }

    let layer = CanvasLayerBuilder::new(200, 200, bouncing_ball)
        .fps(60.0)
        .priority(0)
        .build();

    println!("Created animated layer at 60 FPS");
    println!("Layer priority: {}", layer.priority());
    println!("Target FPS: {}\n", layer.target_fps());

    // ========================================================================
    // Example 3: Multi-layer composition
    // ========================================================================
    println!("Example 3: Multi-Layer Composition");
    println!("-----------------------------------");

    fn background_layer(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(400, 300)
            .draw(DrawOp::Clear(30, 30, 30, 255))
            .draw(DrawOp::Rect {
                x: 0,
                y: 250,
                width: 400,
                height: 50,
                r: 50,
                g: 100,
                b: 50,
                a: 255,
            })
    }

    fn midground_layer(_canvas: &Canvas, delta: f32, _controller: &dyn Controller) -> Canvas {
        static mut TIME: f32 = 0.0;
        unsafe {
            TIME += delta;
        }

        let offset = (unsafe { TIME } * 50.0) as u32 % 400;

        Canvas::new(400, 300)
            .draw(DrawOp::Clear(0, 0, 0, 0)) // Transparent background
            .draw(DrawOp::Rect {
                x: offset,
                y: 220,
                width: 40,
                height: 30,
                r: 100,
                g: 100,
                b: 200,
                a: 255,
            })
    }

    fn ui_layer(_canvas: &Canvas, _delta: f32, _controller: &dyn Controller) -> Canvas {
        Canvas::new(400, 300)
            .draw(DrawOp::Clear(0, 0, 0, 0))
            .draw(DrawOp::Rect {
                x: 10,
                y: 10,
                width: 100,
                height: 30,
                r: 0,
                g: 0,
                b: 0,
                a: 200,
            })
            .draw(DrawOp::HLine {
                x: 15,
                y: 20,
                length: 90,
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            })
    }

    let bg = CanvasLayerBuilder::new(400, 300, background_layer)
        .fps(30.0)
        .priority(0)
        .build();

    let mg = CanvasLayerBuilder::new(400, 300, midground_layer)
        .fps(60.0)
        .priority(5)
        .build();

    let ui = CanvasLayerBuilder::new(400, 300, ui_layer)
        .fps(30.0)
        .priority(10)
        .build();

    let stack = LayerStack::new().with_layer(bg).with_layer(mg).with_layer(ui);

    let frame = Frame {
        number: 0,
        time: 0.0,
        delta: 0.016,
        pixels: Vec::new(),
    };
    let controller = DemoController;
    let context = DisplayContext::new(400, 300);

    let updated_stack = stack.update(&frame, &controller);
    let mask = vec![true; 400 * 300];
    let outputs: Vec<_> = updated_stack.render(&mask, &context).collect();

    println!("Composed {} layers", outputs.len());
    println!(
        "Background: {} pixels",
        outputs[0].pixels.len() / 4
    );
    println!(
        "Midground: {} pixels (with alpha)",
        outputs[1].pixels.len() / 4
    );
    println!("UI: {} pixels (with alpha)\n", outputs[2].pixels.len() / 4);

    // ========================================================================
    // Example 4: Drawing primitives showcase
    // ========================================================================
    println!("Example 4: Drawing Primitives");
    println!("------------------------------");

    let _primitives = Canvas::new(400, 400)
        .draw(DrawOp::Clear(240, 240, 240, 255))
        // Horizontal and vertical lines
        .draw(DrawOp::HLine {
            x: 50,
            y: 50,
            length: 100,
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        })
        .draw(DrawOp::VLine {
            x: 50,
            y: 50,
            length: 100,
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        })
        // Diagonal line
        .draw(DrawOp::Line {
            x1: 200,
            y1: 50,
            x2: 300,
            y2: 150,
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        })
        // Rectangle
        .draw(DrawOp::Rect {
            x: 50,
            y: 200,
            width: 80,
            height: 60,
            r: 255,
            g: 128,
            b: 0,
            a: 255,
        })
        // Circle outline
        .draw(DrawOp::Circle {
            cx: 250,
            cy: 250,
            radius: 40,
            r: 128,
            g: 0,
            b: 128,
            a: 255,
        })
        // Filled circle
        .draw(DrawOp::FilledCircle {
            cx: 250,
            cy: 250,
            radius: 20,
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        })
        .execute_ops();

    println!("Drew all primitive types on {}x{} canvas", 400, 400);
    println!("Total operations executed: 8\n");

    // ========================================================================
    // Example 5: Functional composition pattern
    // ========================================================================
    println!("Example 5: Functional Composition");
    println!("----------------------------------");

    // Chain operations functionally
    let canvas = Canvas::new(100, 100)
        .draw(DrawOp::Clear(0, 0, 0, 255))
        .draw(DrawOp::Rect {
            x: 10,
            y: 10,
            width: 80,
            height: 80,
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        });

    // Clone and add more operations
    let canvas2 = canvas
        .clone()
        .draw(DrawOp::FilledCircle {
            cx: 50,
            cy: 50,
            radius: 30,
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        })
        .execute_ops();

    println!(
        "Functional composition allows reusable canvas configurations"
    );
    println!("Canvas dimensions: {:?}\n", canvas2.dimensions());

    // ========================================================================
    // Example 6: Performance test
    // ========================================================================
    println!("Example 6: Performance");
    println!("----------------------");

    use std::time::Instant;

    let start = Instant::now();
    let mut canvas = Canvas::new(1920, 1080);

    // Queue 1000 draw operations
    for i in 0..1000 {
        let x = (i * 17) % 1920;
        let y = (i * 23) % 1080;
        canvas = canvas.draw(DrawOp::FilledCircle {
            cx: x,
            cy: y,
            radius: 5,
            r: 255,
            g: 128,
            b: 64,
            a: 255,
        });
    }

    let canvas = canvas.execute_ops();
    let elapsed = start.elapsed();

    println!("Drew 1000 circles on 1920x1080 canvas");
    println!("Time: {:?}", elapsed);
    println!("Pixels rendered: {}", canvas.pixels().len() / 4);

    println!("\n✓ All examples completed successfully!");
}
