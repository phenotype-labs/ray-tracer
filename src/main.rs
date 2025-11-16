use ray_tracer::{camera, renderer, cli, frame, window};

use clap::Parser;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window as WinitWindow, WindowId},
};
use camera::Camera;
use renderer::RayTracer;
use frame::{FrameIterator, FrameInfo};
use window::Window;

const FPS_UPDATE_INTERVAL: f32 = 1.0;
const INITIAL_WINDOW_WIDTH: u32 = 600;
const INITIAL_WINDOW_HEIGHT: u32 = 600;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct App {
    window: Option<Window>,
    raytracer: Option<RayTracer>,
    camera: Camera,
    frames: FrameIterator,
    frame_count: u32,
    fps: f32,
    fps_update_timer: f32,
    cursor_position: Option<(f64, f64)>,
    no_ui: bool,
    should_exit: bool,
}

impl App {
    fn new(no_ui: bool) -> Self {
        Self {
            window: None,
            raytracer: None,
            camera: Camera::new(),
            frames: FrameIterator::new(),
            frame_count: 0,
            fps: 0.0,
            fps_update_timer: 0.0,
            cursor_position: None,
            no_ui,
            should_exit: false,
        }
    }

    fn update_fps(&mut self, frame: &FrameInfo) {
        self.frame_count += 1;
        self.fps_update_timer += frame.delta;

        if self.fps_update_timer >= FPS_UPDATE_INTERVAL {
            self.fps = self.frame_count as f32 / self.fps_update_timer;
            if !self.no_ui {
                println!("FPS: {:.1} | Time: {:.2}s", self.fps, frame.time);
            }
            self.frame_count = 0;
            self.fps_update_timer = 0.0;
        }
    }

    fn draw_frame(&mut self, frame: &FrameInfo) {
        if let (Some(window), Some(raytracer)) = (&self.window, &mut self.raytracer) {
            if let Err(e) = window.draw(raytracer, &self.camera, self.fps, frame) {
                eprintln!("Render error: {}", e);
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let winit_window = match event_loop.create_window(
                WinitWindow::default_attributes()
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

            let raytracer = match pollster::block_on(RayTracer::new(winit_window.clone(), self.no_ui)) {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to initialize ray tracer: {}", e);
                    event_loop.exit();
                    return;
                }
            };

            self.window = Some(Window::new(winit_window));
            self.raytracer = Some(raytracer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let (Some(raytracer), Some(window)) = (&mut self.raytracer, &self.window) {
            if raytracer.handle_event(window.inner(), &event) {
                return;
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
            } => self.should_exit = true,
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                if let (Some(raytracer), Some(cursor_pos)) = (&mut self.raytracer, self.cursor_position) {
                    raytracer.set_debug_pixel(cursor_pos.0 as u32, cursor_pos.1 as u32);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.camera.process_keyboard(&event),
            WindowEvent::RedrawRequested => {
                // Get next frame from iterator
                let frame = self.frames.next().unwrap();

                self.update_fps(&frame);
                self.camera.update();

                if let (Some(raytracer), Some(window)) = (&mut self.raytracer, &self.window) {
                    if raytracer.needs_reload() {
                        let new_scene = raytracer.get_current_scene();
                        if !self.no_ui {
                            println!("Reloading scene: {}", new_scene);
                        }
                        std::env::set_var("SCENE", &new_scene);

                        match pollster::block_on(RayTracer::new(window.inner().clone(), self.no_ui)) {
                            Ok(new_raytracer) => {
                                *raytracer = new_raytracer;
                                self.camera = Camera::new();
                            }
                            Err(e) => {
                                eprintln!("Failed to reload scene: {}", e);
                            }
                        }
                    }
                }

                // Draw the frame using iterator pattern
                self.draw_frame(&frame);
            }
            _ => {}
        }

        if self.should_exit {
            event_loop.exit();
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

    let args = cli::Cli::parse();
    let no_ui = args.no_ui;

    let event_loop = EventLoop::new()?;
    let mut app = App::new(no_ui);

    if !no_ui {
        println!("Ray Tracer - Controls: WASD (move), Q/E (rotate), Space/Shift (up/down), Escape to quit");
    }
    event_loop.run_app(&mut app)?;

    Ok(())
}
