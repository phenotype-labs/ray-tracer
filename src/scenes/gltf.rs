use crate::loaders::gltf::load_gltf_with_animation;
use crate::types::BoxData;

/// Creates a scene by loading a glTF file
/// The file path can be specified via the GLTF_FILE environment variable,
/// or defaults to "models/no_animation/scene.gltf"
pub fn create_gltf_scene() -> Vec<BoxData> {
    let file_path =
        std::env::var("GLTF_FILE").unwrap_or_else(|_| "models/no_animation/scene.gltf".to_string());

    println!("Loading glTF file: {}", file_path);

    match load_gltf_with_animation(&file_path) {
        Ok((boxes, animation_data)) => {
            println!("Successfully loaded {} boxes from glTF file", boxes.len());

            if let Some(anim) = animation_data {
                println!("Animation loaded: {} (duration: {:.2}s)", anim.name, anim.duration);
            } else {
                println!("No animations in this glTF file");
            }

            boxes
        }
        Err(e) => {
            eprintln!("Failed to load glTF file: {}", e);
            eprintln!("Error details: {:?}", e);

            // Return a simple error indicator scene
            vec![
                BoxData::new(
                    [-1.0, 0.0, -1.0],
                    [1.0, 0.1, 1.0],
                    [0.8, 0.8, 0.8], // Gray ground
                ),
                BoxData::new(
                    [-0.5, 0.1, -0.5],
                    [0.5, 1.1, 0.5],
                    [1.0, 0.0, 0.0], // Red error box
                ),
            ]
        }
    }
}
