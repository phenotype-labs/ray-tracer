pub mod gltf;
pub mod gltf_triangles;

pub use gltf::{load_gltf_file, load_gltf_with_animation, AnimationData};
pub use gltf_triangles::{load_gltf_triangles, GltfScene, TextureData};
