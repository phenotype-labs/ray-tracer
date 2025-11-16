pub mod camera;
pub mod cli;
pub mod core;
pub mod demo;
pub mod display;
pub mod frame;
pub mod grid;
pub mod grid_triangles;
pub mod loaders;
pub mod math;
pub mod renderer;
pub mod scenes;
pub mod types;
pub mod window;

// Re-export scene functions for backward compatibility
pub use scenes::{
    create_composed_scene, create_default_scene, create_fractal_scene, create_gltf_scene,
    create_reflected_scene, create_tunnel_scene, create_walls_scene,
};
