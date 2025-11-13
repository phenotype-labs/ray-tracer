use glam::Vec3;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};
use crate::types::CameraUniform;

pub const CAMERA_SPEED: f32 = 0.1;
pub const CAMERA_ROTATION_SPEED: f32 = 0.05;

#[derive(Default, Clone, Copy)]
pub struct MovementState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub rotate_left: bool,
    pub rotate_right: bool,
}

impl MovementState {
    const fn to_direction(&self, positive: bool, negative: bool) -> f32 {
        match (positive, negative) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        }
    }

    const fn velocity(&self) -> (f32, f32, f32) {
        (
            self.to_direction(self.forward, self.backward),
            self.to_direction(self.right, self.left),
            self.to_direction(self.up, self.down),
        )
    }

    const fn rotation_velocity(&self) -> f32 {
        self.to_direction(self.rotate_right, self.rotate_left)
    }
}

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub movement: MovementState,
}

impl Camera {
    pub fn new() -> Self {
        let scene_name = std::env::var("SCENE").unwrap_or_else(|_| "fractal".to_string());

        let (position, yaw, pitch) = match scene_name.as_str() {
            "walls" => (Vec3::new(0.0, 5.0, 0.0), 0.0, 0.0),
            "tunnel" => (Vec3::new(0.0, 0.0, 20.0), std::f32::consts::PI, 0.0),
            _ => (Vec3::new(0.0, 8.0, 15.0), std::f32::consts::PI, -0.6),
        };

        Self {
            position,
            yaw,
            pitch,
            movement: MovementState::default(),
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        Vec3::Y
    }

    pub fn update(&mut self) {
        let (fwd, right_dir, up_dir) = self.movement.velocity();

        let displacement = self.forward() * fwd * CAMERA_SPEED
            + self.right() * right_dir * CAMERA_SPEED
            + Vec3::Y * up_dir * CAMERA_SPEED;

        self.position += displacement;
        self.yaw += self.movement.rotation_velocity() * CAMERA_ROTATION_SPEED;
    }

    pub fn to_uniform(&self, time: f32, screen_height: f32, fov: f32) -> CameraUniform {
        let lod_factor = screen_height / (2.0 * (fov / 2.0).tan());
        let min_pixel_size = 2.0;

        CameraUniform {
            position: self.position.to_array(),
            _pad1: 0.0,
            forward: self.forward().to_array(),
            _pad2: 0.0,
            right: self.right().to_array(),
            _pad3: 0.0,
            up: self.up().to_array(),
            time,
            lod_factor,
            min_pixel_size,
            _pad4: [0.0, 0.0],
        }
    }

    pub fn process_keyboard(&mut self, event: &KeyEvent) {
        let is_pressed = event.state.is_pressed();
        if let PhysicalKey::Code(keycode) = event.physical_key {
            match keycode {
                KeyCode::KeyW => self.movement.forward = is_pressed,
                KeyCode::KeyS => self.movement.backward = is_pressed,
                KeyCode::KeyA => self.movement.left = is_pressed,
                KeyCode::KeyD => self.movement.right = is_pressed,
                KeyCode::Space => self.movement.up = is_pressed,
                KeyCode::ShiftLeft => self.movement.down = is_pressed,
                KeyCode::KeyQ => self.movement.rotate_left = is_pressed,
                KeyCode::KeyE => self.movement.rotate_right = is_pressed,
                _ => {}
            }
        }
    }
}
