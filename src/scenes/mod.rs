mod common;
mod composed;
mod fractal;
mod walls;
mod tunnel;
mod default;
mod reflected;

pub use composed::create_composed_scene;
pub use fractal::create_fractal_scene;
pub use walls::create_walls_scene;
pub use tunnel::create_tunnel_scene;
pub use default::create_default_scene;
pub use reflected::create_reflected_scene;
