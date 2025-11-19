use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use ray_tracer::core::*;

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<Arc<GpuContext>>,
    surface_renderer: Option<SurfaceRenderer>,
    layers: Option<LayerStack>,
    controller: WinitController,
    last_update: Instant,
    frame_count: u32,
    fps_timer: f32,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            gpu: None,
            surface_renderer: None,
            layers: None,
            controller: WinitController::new(),
            last_update: Instant::now(),
            frame_count: 0,
            fps_timer: 0.0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Create window
        let window_attributes = Window::default_attributes()
            .with_title("Ray Tracing Layer Example")
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        // Initialize GPU context and renderer
        let gpu = pollster::block_on(async {
            let surface = {
                let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::PRIMARY,
                    ..Default::default()
                });
                instance.create_surface(window.clone()).unwrap()
            };

            Arc::new(GpuContext::new_with_surface(&surface).await.unwrap())
        });

        let surface_renderer =
            SurfaceRenderer::new(window.clone(), gpu.clone()).expect("Failed to create renderer");

        // Create ray tracing layer
        let size = window.inner_size();
        let scene = std::env::var("SCENE").unwrap_or_else(|_| "pyramid".to_string());

        let rt_layer = pollster::block_on(async {
            RayTracingLayerBuilder::new(gpu.clone(), &scene, size.width, size.height)
                .fps(60.0)
                .priority(0)
                .build()
                .await
                .expect("Failed to create ray tracing layer")
        });

        // Create layer stack
        let mut layers = LayerStack::new();
        layers = layers.add_layer(rt_layer);

        println!("Ray Tracing Layer Example initialized");
        println!("Scene: {}", scene);
        println!("Controls:");
        println!("  WASD - Move camera");
        println!("  Q/E - Rotate camera");
        println!("  Space/Shift - Move up/down");
        println!("  ESC - Exit");

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.surface_renderer = Some(surface_renderer);
        self.layers = Some(layers);
        self.last_update = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Process input
        self.controller.process_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) =
                    event.physical_key
                {
                    if event.state.is_pressed() {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                let surface_renderer = self.surface_renderer.as_ref().unwrap();
                let layers = self.layers.as_mut().unwrap();

                // Calculate delta time
                let now = Instant::now();
                let delta = now.duration_since(self.last_update).as_secs_f32();
                self.last_update = now;

                // Update FPS counter
                self.frame_count += 1;
                self.fps_timer += delta;
                if self.fps_timer >= 1.0 {
                    println!("FPS: {}", self.frame_count);
                    self.frame_count = 0;
                    self.fps_timer = 0.0;
                }

                // Update layers
                *layers = layers.update(delta, &self.controller);

                // Render layers
                let size = window.inner_size();
                let context = DisplayContext {
                    width: size.width,
                    height: size.height,
                };

                let outputs = layers.render(&context);

                // Display on surface
                if let Some(output) = outputs.first() {
                    if let Err(e) = surface_renderer.render(output) {
                        eprintln!("Render error: {}", e);
                    }
                }

                // Reset per-frame input state
                self.controller.reset_deltas();

                // Request next frame
                window.request_redraw();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = self.surface_renderer.as_mut() {
                    renderer.resize(new_size.width, new_size.height);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
