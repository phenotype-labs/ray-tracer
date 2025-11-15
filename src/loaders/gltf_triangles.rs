use anyhow::{Context, Result};
use glam::Vec3;
use std::path::Path;

use crate::types::{TriangleData, MaterialData};

/// glTF scene data with triangles, materials, and textures
pub struct GltfScene {
    pub triangles: Vec<TriangleData>,
    pub materials: Vec<MaterialData>,
    pub textures: Vec<TextureData>,
}

/// Texture data loaded from glTF
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,  // RGBA8
}

/// Loads a glTF file and extracts triangles with UVs and materials
pub fn load_gltf_triangles(path: impl AsRef<Path>) -> Result<GltfScene> {
    let path = path.as_ref();
    println!("Loading glTF file for triangle rendering: {:?}", path);

    let (gltf, buffers, images) = gltf::import(path)
        .context(format!("Failed to load glTF file: {:?}", path))?;

    println!("glTF loaded:");
    println!("  Scenes: {}", gltf.scenes().count());
    println!("  Nodes: {}", gltf.nodes().count());
    println!("  Meshes: {}", gltf.meshes().count());
    println!("  Materials: {}", gltf.materials().count());
    println!("  Images: {}", images.len());

    let mut all_triangles = Vec::new();
    let mut materials = Vec::new();
    let mut textures = Vec::new();

    // Load materials
    for (mat_idx, material) in gltf.materials().enumerate() {
        let pbr = material.pbr_metallic_roughness();
        let base_color = pbr.base_color_factor();

        let texture_index = if let Some(info) = pbr.base_color_texture() {
            let tex_index = info.texture().index();
            println!("  Material {} uses texture {}", mat_idx, tex_index);
            tex_index as i32
        } else {
            -1
        };

        let material_data = if texture_index >= 0 {
            MaterialData::new_textured(base_color, texture_index as u32)
        } else {
            MaterialData::new_color(base_color)
        };

        materials.push(material_data);
    }

    // Add default material if none exist
    if materials.is_empty() {
        materials.push(MaterialData::new_color([0.7, 0.7, 0.7, 1.0]));
    }

    // Load textures
    for (img_idx, image) in images.iter().enumerate() {
        println!("  Loading texture {}: {}x{} ({:?})",
            img_idx, image.width, image.height, image.format);

        let rgba_data = match image.format {
            gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
            gltf::image::Format::R8G8B8 => {
                // Convert RGB to RGBA
                let mut rgba = Vec::with_capacity(image.pixels.len() / 3 * 4);
                for rgb in image.pixels.chunks(3) {
                    rgba.extend_from_slice(rgb);
                    rgba.push(255); // Alpha
                }
                rgba
            }
            gltf::image::Format::R8G8 => {
                // Convert RG (2 channels) to RGBA
                // R8G8 is typically used for normal maps or other 2-channel data
                let mut rgba = Vec::with_capacity(image.pixels.len() / 2 * 4);
                for rg in image.pixels.chunks(2) {
                    rgba.push(rg[0]); // R
                    rgba.push(rg[1]); // G
                    rgba.push(0);     // B (set to 0)
                    rgba.push(255);   // Alpha
                }
                rgba
            }
            _ => {
                println!("    Warning: Unsupported texture format {:?}, using default", image.format);
                vec![255; (image.width * image.height * 4) as usize]
            }
        };

        textures.push(TextureData {
            width: image.width,
            height: image.height,
            data: rgba_data,
        });
    }

    // Process each scene
    for scene in gltf.scenes() {
        println!("Processing scene: {:?}", scene.name());

        for node in scene.nodes() {
            process_node_triangles(&node, &buffers, &glam::Mat4::IDENTITY, &mut all_triangles)?;
        }
    }

    println!("Extracted {} triangles from glTF", all_triangles.len());
    println!("Loaded {} materials", materials.len());
    println!("Loaded {} textures", textures.len());

    Ok(GltfScene {
        triangles: all_triangles,
        materials,
        textures,
    })
}

/// Recursively processes glTF nodes to extract triangles
fn process_node_triangles(
    node: &gltf::Node,
    buffers: &[gltf::buffer::Data],
    parent_transform: &glam::Mat4,
    triangles: &mut Vec<TriangleData>,
) -> Result<()> {
    let local_transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
    let global_transform = *parent_transform * local_transform;

    if let Some(mesh) = node.mesh() {
        process_mesh_triangles(&mesh, buffers, &global_transform, triangles)?;
    }

    for child in node.children() {
        process_node_triangles(&child, buffers, &global_transform, triangles)?;
    }

    Ok(())
}

/// Processes a glTF mesh and extracts triangles
fn process_mesh_triangles(
    mesh: &gltf::Mesh,
    buffers: &[gltf::buffer::Data],
    transform: &glam::Mat4,
    triangles: &mut Vec<TriangleData>,
) -> Result<()> {
    println!("  Processing mesh: {:?}", mesh.name());

    for primitive in mesh.primitives() {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        // Extract positions
        let positions = reader
            .read_positions()
            .context("Mesh primitive has no positions")?;

        let vertices: Vec<Vec3> = positions
            .map(|pos| {
                let v = Vec3::from_array(pos);
                transform.transform_point3(v)
            })
            .collect();

        // Extract UVs (texture coordinates)
        let uvs: Vec<[f32; 2]> = if let Some(uv_reader) = reader.read_tex_coords(0) {
            uv_reader.into_f32().collect()
        } else {
            // Default UVs if none provided
            vec![[0.0, 0.0]; vertices.len()]
        };

        // Get material index
        let material_id = primitive.material().index().unwrap_or(0) as u32;

        // Extract indices and create triangles
        if let Some(indices) = reader.read_indices() {
            let indices: Vec<u32> = indices.into_u32().collect();

            for tri_indices in indices.chunks(3) {
                if tri_indices.len() == 3 {
                    let i0 = tri_indices[0] as usize;
                    let i1 = tri_indices[1] as usize;
                    let i2 = tri_indices[2] as usize;

                    let triangle = TriangleData::new(
                        vertices[i0].to_array(),
                        vertices[i1].to_array(),
                        vertices[i2].to_array(),
                        uvs[i0],
                        uvs[i1],
                        uvs[i2],
                        material_id,
                    );

                    triangles.push(triangle);
                }
            }
        } else {
            // No indices - treat as triangle list
            for i in (0..vertices.len()).step_by(3) {
                if i + 2 < vertices.len() {
                    let triangle = TriangleData::new(
                        vertices[i].to_array(),
                        vertices[i + 1].to_array(),
                        vertices[i + 2].to_array(),
                        uvs[i],
                        uvs[i + 1],
                        uvs[i + 2],
                        material_id,
                    );

                    triangles.push(triangle);
                }
            }
        }
    }

    Ok(())
}
