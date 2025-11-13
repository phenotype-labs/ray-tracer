mod types;
mod grid;
mod camera;
mod scene;
mod renderer;

use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
use camera::Camera;
use renderer::RayTracer;

const FPS_UPDATE_INTERVAL: f32 = 1.0;
const INITIAL_WINDOW_WIDTH: u32 = 800;
const INITIAL_WINDOW_HEIGHT: u32 = 600;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct App {
    window: Option<Arc<Window>>,
    raytracer: Option<RayTracer>,
    camera: Camera,
    last_frame_time: Instant,
    frame_count: u32,
    fps: f32,
    fps_update_timer: f32,
    time: f32, // Animation time for moving objects
    start_time: Instant,
}

impl App {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            window: None,
            raytracer: None,
            camera: Camera::new(),
            last_frame_time: now,
            frame_count: 0,
            fps: 0.0,
            fps_update_timer: 0.0,
            time: 0.0,
            start_time: now,
        }
    }

    fn update_fps(&mut self, delta: f32) {
        self.frame_count += 1;
        self.fps_update_timer += delta;

        if self.fps_update_timer >= FPS_UPDATE_INTERVAL {
            self.fps = self.frame_count as f32 / self.fps_update_timer;
            println!("FPS: {:.1} | Time: {:.2}s", self.fps, self.time);
            self.frame_count = 0;
            self.fps_update_timer = 0.0;
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = match event_loop.create_window(
                Window::default_attributes()
                    .with_title("Ray Tracer")
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        INITIAL_WINDOW_WIDTH,
                        INITIAL_WINDOW_HEIGHT,
                    )),
            ) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    eprintln!("Failed to create window: {}", e);
                    event_loop.exit();
                    return;
                }
            };

            let raytracer = match pollster::block_on(RayTracer::new(window.clone())) {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to initialize ray tracer: {}", e);
                    event_loop.exit();
                    return;
                }
            };

            self.window = Some(window);
            self.raytracer = Some(raytracer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle the event first
        if let (Some(raytracer), Some(window)) = (&mut self.raytracer, &self.window) {
            if raytracer.handle_event(window, &event) {
                return; // egui consumed the event
            }
        }

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => self.camera.process_keyboard(&event),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta = now.duration_since(self.last_frame_time).as_secs_f32();
                self.last_frame_time = now;
                self.time = now.duration_since(self.start_time).as_secs_f32();

                self.update_fps(delta);
                self.camera.update();

                if let (Some(raytracer), Some(window)) = (&mut self.raytracer, &self.window) {
                    if let Err(e) = raytracer.render(&self.camera, window, self.fps, self.time) {
                        eprintln!("Render error: {}", e);
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    let mut app = App::new();

    println!("Ray Tracer - Controls: WASD (move), Q/E (rotate), Space/Shift (up/down), Escape to quit");
    event_loop.run_app(&mut app)?;

    Ok(())
}
