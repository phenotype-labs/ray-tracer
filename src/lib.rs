pub mod camera;
pub mod grid;
pub mod math;
pub mod renderer;
pub mod scenes;
pub mod types;

// Re-export scene functions for backward compatibility
pub use scenes::{
    create_default_scene, create_fractal_scene, create_reflected_scene, create_tunnel_scene,
    create_walls_scene,
};
