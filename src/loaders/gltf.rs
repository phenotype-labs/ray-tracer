use anyhow::{Context, Result};
use glam::Vec3;
use std::path::Path;

use crate::math::AABB;
use crate::types::BoxData;

/// Loads a glTF file and converts it to BoxData for the ray tracer
pub fn load_gltf_file(path: impl AsRef<Path>) -> Result<Vec<BoxData>> {
    let path = path.as_ref();
    println!("Loading glTF file: {:?}", path);

    let (gltf, buffers, _images) = gltf::import(path)
        .context(format!("Failed to load glTF file: {:?}", path))?;

    println!("glTF loaded successfully:");
    println!("  Scenes: {}", gltf.scenes().count());
    println!("  Nodes: {}", gltf.nodes().count());
    println!("  Meshes: {}", gltf.meshes().count());
    println!("  Animations: {}", gltf.animations().count());

    let mut all_boxes = Vec::new();

    // Process each scene
    for scene in gltf.scenes() {
        println!("Processing scene: {:?}", scene.name());

        // Process each node in the scene
        for node in scene.nodes() {
            process_node(&node, &buffers, &glam::Mat4::IDENTITY, &mut all_boxes)?;
        }
    }

    if all_boxes.is_empty() {
        println!("Warning: No geometry found in glTF file");
        // Return a placeholder
        all_boxes.push(BoxData::new(
            [-0.5, -0.5, -0.5],
            [0.5, 0.5, 0.5],
            [1.0, 0.0, 1.0], // Magenta to indicate no geometry
        ));
    }

    println!("Extracted {} boxes from glTF", all_boxes.len());
    Ok(all_boxes)
}

/// Recursively processes glTF nodes
fn process_node(
    node: &gltf::Node,
    buffers: &[gltf::buffer::Data],
    parent_transform: &glam::Mat4,
    boxes: &mut Vec<BoxData>,
) -> Result<()> {
    // Compute node transform
    let local_transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
    let global_transform = *parent_transform * local_transform;

    // Process mesh if present
    if let Some(mesh) = node.mesh() {
        process_mesh(&mesh, buffers, &global_transform, boxes)?;
    }

    // Recursively process children
    for child in node.children() {
        process_node(&child, buffers, &global_transform, boxes)?;
    }

    Ok(())
}

/// Processes a glTF mesh
fn process_mesh(
    mesh: &gltf::Mesh,
    buffers: &[gltf::buffer::Data],
    transform: &glam::Mat4,
    boxes: &mut Vec<BoxData>,
) -> Result<()> {
    println!("  Processing mesh: {:?}", mesh.name());

    for primitive in mesh.primitives() {
        // Extract vertices
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        let positions = reader
            .read_positions()
            .context("Mesh primitive has no positions")?;

        let vertices: Vec<Vec3> = positions
            .map(|pos| {
                let v = Vec3::from_array(pos);
                transform.transform_point3(v)
            })
            .collect();

        if vertices.is_empty() {
            continue;
        }

        // Get material color (default to gray)
        let material = primitive.material().pbr_metallic_roughness().base_color_factor();
        let color = [material[0], material[1], material[2]];

        // Check if we have indices
        if let Some(indices) = reader.read_indices() {
            // Convert to triangles and create AABBs
            let indices: Vec<u32> = indices.into_u32().collect();

            // Convert mesh to AABBs (one per triangle)
            for triangle in indices.chunks(3) {
                if triangle.len() == 3 {
                    let v0 = vertices[triangle[0] as usize];
                    let v1 = vertices[triangle[1] as usize];
                    let v2 = vertices[triangle[2] as usize];

                    // Compute AABB for this triangle
                    let min = v0.min(v1).min(v2);
                    let max = v0.max(v1).max(v2);

                    boxes.push(BoxData::new(min.to_array(), max.to_array(), color));
                }
            }
        } else {
            // No indices - treat as triangle list
            for triangle in vertices.chunks(3) {
                if triangle.len() == 3 {
                    let min = triangle[0].min(triangle[1]).min(triangle[2]);
                    let max = triangle[0].max(triangle[1]).max(triangle[2]);

                    boxes.push(BoxData::new(min.to_array(), max.to_array(), color));
                }
            }
        }
    }

    Ok(())
}

/// Loads glTF with animation support
/// Returns (static boxes, animation data)
pub fn load_gltf_with_animation(path: impl AsRef<Path>) -> Result<(Vec<BoxData>, Option<AnimationData>)> {
    let path = path.as_ref();
    println!("Loading glTF file with animation: {:?}", path);

    let (gltf, buffers, _images) = gltf::import(path)
        .context(format!("Failed to load glTF file: {:?}", path))?;

    let animation_count = gltf.animations().count();
    println!("Found {} animations", animation_count);

    // Load static geometry
    let boxes = load_gltf_file(path)?;

    // Load animation data if present
    let animation_data = if animation_count > 0 {
        if let Some(animation) = gltf.animations().next() {
            println!("Loading animation: {:?}", animation.name());

            Some(AnimationData {
                name: animation.name().unwrap_or("unnamed").to_string(),
                duration: calculate_animation_duration(&animation, &buffers),
            })
        } else {
            eprintln!("Warning: Expected {} animation(s) but none accessible", animation_count);
            None
        }
    } else {
        None
    };

    Ok((boxes, animation_data))
}

/// Animation data structure
#[derive(Debug, Clone)]
pub struct AnimationData {
    pub name: String,
    pub duration: f32,
}

/// Calculates animation duration
fn calculate_animation_duration(animation: &gltf::Animation, buffers: &[gltf::buffer::Data]) -> f32 {
    let mut max_time = 0.0f32;

    for channel in animation.channels() {
        let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

        if let Some(inputs) = reader.read_inputs() {
            for time in inputs {
                max_time = max_time.max(time);
            }
        }
    }

    max_time
}

/// Computes overall bounding box for vertices
pub fn compute_mesh_bounds(vertices: &[Vec3]) -> AABB {
    if vertices.is_empty() {
        return AABB {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        };
    }

    let mut min = vertices[0];
    let mut max = vertices[0];

    for &vertex in vertices.iter().skip(1) {
        min = min.min(vertex);
        max = max.max(vertex);
    }

    AABB { min, max }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_mesh_bounds() {
        let vertices = vec![
            Vec3::new(-1.0, -2.0, -3.0),
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
        ];

        let bounds = compute_mesh_bounds(&vertices);

        assert_eq!(bounds.min, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bounds.max, Vec3::new(1.0, 2.0, 3.0));
    }
}
