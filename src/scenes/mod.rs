mod common;
mod composed;
mod fractal;
mod walls;
mod tunnel;
mod default;
mod reflected;
mod gltf;
mod pyramid;

pub use composed::create_composed_scene;
pub use fractal::create_fractal_scene;
pub use walls::create_walls_scene;
pub use tunnel::create_tunnel_scene;
pub use default::create_default_scene;
pub use reflected::create_reflected_scene;
pub use gltf::{create_gltf_scene, create_gltf_triangles};
pub use pyramid::{create_pyramid_scene, create_pyramid_triangles};
