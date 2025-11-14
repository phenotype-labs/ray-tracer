use crate::loaders::gltf_triangles::load_gltf_triangles;
use crate::types::{TriangleData, MaterialData};

pub struct TriangleScene {
    pub triangles: Vec<TriangleData>,
    pub materials: Vec<MaterialData>,
}

/// Creates a triangle scene by loading a glTF file
/// This returns actual triangles, not AABBs
pub fn create_gltf_triangle_scene() -> TriangleScene {
    let file_path =
        std::env::var("GLTF_FILE").unwrap_or_else(|_| "models/no_animation/scene.gltf".to_string());

    println!("Loading glTF file for triangle rendering: {}", file_path);

    match load_gltf_triangles(&file_path) {
        Ok(scene) => {
            println!("Successfully loaded {} triangles from glTF file", scene.triangles.len());
            println!("Loaded {} materials", scene.materials.len());

            TriangleScene {
                triangles: scene.triangles,
                materials: scene.materials,
            }
        }
        Err(e) => {
            eprintln!("Failed to load glTF file for triangles: {}", e);
            eprintln!("Error details: {:?}", e);

            // Return a simple test triangle
            let tri = TriangleData::new(
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0],
                [1.0, 0.0],
                [0.5, 1.0],
                0,
            );

            TriangleScene {
                triangles: vec![tri],
                materials: vec![MaterialData::new_color([1.0, 0.0, 0.0, 1.0])],
            }
        }
    }
}
