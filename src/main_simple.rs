use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
use wgpu::util::DeviceExt;
use glam::Vec3;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    position: [f32; 3],
    _pad1: f32,
    forward: [f32; 3],
    _pad2: f32,
    right: [f32; 3],
    _pad3: f32,
    up: [f32; 3],
    _pad4: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BoxData {
    min: [f32; 3],
    _pad1: f32,
    max: [f32; 3],
    _pad2: f32,
    color: [f32; 3],
    _pad3: f32,
}

struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 5.0),
            yaw: std::f32::consts::PI,
            pitch: -0.3,
            speed: 0.1,
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        ).normalize()
    }

    fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    fn up(&self) -> Vec3 {
        Vec3::Y
    }

    fn update(&mut self) {
        let forward = self.forward();
        let right = self.right();

        if self.move_forward {
            self.position += forward * self.speed;
        }
        if self.move_backward {
            self.position -= forward * self.speed;
        }
        if self.move_right {
            self.position += right * self.speed;
        }
        if self.move_left {
            self.position -= right * self.speed;
        }
        if self.move_up {
            self.position.y += self.speed;
        }
        if self.move_down {
            self.position.y -= self.speed;
        }
    }

    fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            position: self.position.to_array(),
            _pad1: 0.0,
            forward: self.forward().to_array(),
            _pad2: 0.0,
            right: self.right().to_array(),
            _pad3: 0.0,
            up: self.up().to_array(),
            _pad4: 0.0,
        }
    }

    fn process_keyboard(&mut self, event: &KeyEvent) {
        let is_pressed = event.state.is_pressed();
        if let PhysicalKey::Code(keycode) = event.physical_key {
            match keycode {
                KeyCode::KeyW => self.move_forward = is_pressed,
                KeyCode::KeyS => self.move_backward = is_pressed,
                KeyCode::KeyA => self.move_left = is_pressed,
                KeyCode::KeyD => self.move_right = is_pressed,
                KeyCode::Space => self.move_up = is_pressed,
                KeyCode::ShiftLeft => self.move_down = is_pressed,
                _ => {}
            }
        }
    }
}

struct RayTracer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    output_texture: wgpu::Texture,
    output_texture_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
}
