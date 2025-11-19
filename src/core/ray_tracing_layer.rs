use std::sync::Arc;
use glam::Vec3;
use wgpu::util::DeviceExt;

use super::controller::{Button, Controller};
use super::display_context::DisplayContext;
use super::gpu_context::GpuContext;
use super::layer::{Layer, LayerLogic, LayerOutput, TimedLayer};

use crate::camera::{CAMERA_SPEED, CAMERA_ROTATION_SPEED};
use crate::grid::HierarchicalGrid;
use crate::scenes::*;
use crate::types::{CameraUniform, MaterialData};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const WORKGROUP_SIZE: u32 = 8;
const DEFAULT_FOV: f32 = std::f32::consts::FRAC_PI_4; // π/4 = 45 degrees

/// Functional camera state for ray tracing
#[derive(Clone, Debug)]
struct CameraState {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

impl CameraState {
    /// Create camera for a specific scene
    fn new_for_scene(scene_name: &str) -> Self {
        let (position, yaw, pitch) = match scene_name {
            "composed" => (Vec3::new(0.0, 40.0, 40.0), std::f32::consts::PI, -0.7),
            "walls" => (Vec3::new(0.0, 5.0, 0.0), 0.0, 0.0),
            "tunnel" => (Vec3::new(0.0, 0.0, 20.0), std::f32::consts::PI, 0.0),
            "gltf" => (Vec3::new(200.0, 200.0, 300.0), 3.35, -0.28),
            "pyramid" => (Vec3::new(0.0, 8.0, 20.0), std::f32::consts::PI, -0.5),
            _ => (Vec3::new(0.0, 8.0, 15.0), std::f32::consts::PI, -0.6),
        };

        Self {
            position,
            yaw,
            pitch,
        }
    }

    /// Functional update from controller input
    fn update(&self, delta: f32, controller: &dyn Controller) -> Self {
        // Calculate movement velocity
        let mut fwd = 0.0f32;
        let mut right_dir = 0.0f32;
        let mut up_dir = 0.0f32;

        if controller.is_down(Button::KeyW) {
            fwd += 1.0;
        }
        if controller.is_down(Button::KeyS) {
            fwd -= 1.0;
        }
        if controller.is_down(Button::KeyD) {
            right_dir += 1.0;
        }
        if controller.is_down(Button::KeyA) {
            right_dir -= 1.0;
        }
        if controller.is_down(Button::Space) {
            up_dir += 1.0;
        }
        if controller.is_down(Button::Shift) {
            up_dir -= 1.0;
        }

        // Calculate rotation velocity
        let mut yaw_delta = 0.0f32;
        if controller.is_down(Button::KeyE) {
            yaw_delta += 1.0;
        }
        if controller.is_down(Button::KeyQ) {
            yaw_delta -= 1.0;
        }

        // Calculate displacement
        let forward = self.forward();
        let right = self.right();

        let displacement = forward * fwd * CAMERA_SPEED * delta
            + right * right_dir * CAMERA_SPEED * delta
            + Vec3::Y * up_dir * CAMERA_SPEED * delta;

        Self {
            position: self.position + displacement,
            yaw: self.yaw + yaw_delta * CAMERA_ROTATION_SPEED * delta,
            pitch: self.pitch,
        }
    }

    /// Get forward vector
    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    /// Get right vector
    fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// Get up vector
    fn up(&self) -> Vec3 {
        Vec3::Y
    }

    /// Convert to GPU uniform
    fn to_uniform(&self, time: f32, screen_height: f32, fov: f32, show_grid: bool) -> CameraUniform {
        let lod_factor = Self::calculate_lod_factor(screen_height, fov);
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
            show_grid: if show_grid { 1.0 } else { 0.0 },
            _pad4: 0.0,
        }
    }

    fn calculate_lod_factor(screen_height: f32, fov: f32) -> f32 {
        screen_height / (2.0 * (fov / 2.0).tan())
    }
}

/// GPU compute state for ray tracing
struct ComputeState {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    output_texture: wgpu::Texture,
    staging_buffer: wgpu::Buffer,
    width: u32,
    height: u32,
}

impl ComputeState {
    async fn new(
        gpu: &GpuContext,
        scene_name: &str,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let device = gpu.device();

        // Load scene data
        let boxes = match scene_name {
            "composed" => create_composed_scene(),
            "walls" => create_walls_scene(),
            "tunnel" => create_tunnel_scene(),
            "default" => create_default_scene(),
            "reflected" => create_reflected_scene(),
            "gltf" => vec![],
            "pyramid" => vec![],
            _ => create_fractal_scene(),
        };

        // Load triangles and materials
        let (triangles, materials, _textures) = if scene_name == "pyramid" {
            let tris = create_pyramid_triangles();
            let mats = vec![
                MaterialData::new_color([1.0, 0.2, 0.2, 1.0]), // Red
                MaterialData::new_color([0.2, 1.0, 0.2, 1.0]), // Green
                MaterialData::new_color([0.2, 0.2, 1.0, 1.0]), // Blue
                MaterialData::new_color([1.0, 1.0, 0.2, 1.0]), // Yellow
                MaterialData::new_color([0.5, 0.5, 0.5, 1.0]), // Gray
            ];
            (tris, mats, vec![])
        } else if scene_name == "gltf" {
            create_gltf_triangles()
        } else {
            (vec![], vec![], vec![])
        };

        // Build hierarchical grid
        let grid = HierarchicalGrid::build(&boxes, &triangles);
        let (metadata, coarse_counts, fine_cells) = grid.to_gpu_buffers();

        // Create GPU buffers
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let boxes_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boxes Buffer"),
            contents: bytemuck::cast_slice(&boxes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let triangles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Triangles Buffer"),
            contents: bytemuck::cast_slice(&triangles),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let materials_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials Buffer"),
            contents: bytemuck::cast_slice(&materials),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let grid_metadata_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Metadata Buffer"),
            contents: bytemuck::cast_slice(&[metadata]),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let coarse_counts_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Coarse Counts Buffer"),
            contents: bytemuck::cast_slice(&coarse_counts),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let fine_cells_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fine Cells Buffer"),
            contents: bytemuck::cast_slice(&fine_cells),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create output texture
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ray Tracing Output Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (width * height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Load compute shader
        let shader_source = include_str!("../raytracer_unified.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ray Tracing Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Ray Tracing Bind Group Layout"),
            entries: &[
                // Camera (binding 0)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output texture (binding 1)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Boxes (binding 2)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Triangles (binding 3)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Materials (binding 4)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Grid metadata (binding 5)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Coarse counts (binding 6)
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Fine cells (binding 7)
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ray Tracing Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: boxes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: triangles_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: materials_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: grid_metadata_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: coarse_counts_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: fine_cells_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ray Tracing Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Ray Tracing Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            pipeline,
            bind_group,
            camera_buffer,
            output_texture,
            staging_buffer,
            width,
            height,
        })
    }

    /// Render a frame and return pixels
    fn render(
        &self,
        gpu: &GpuContext,
        camera: &CameraState,
        time: f32,
    ) -> Result<Vec<u8>> {
        let device = gpu.device();
        let queue = gpu.queue();

        // Update camera uniform
        let camera_uniform = camera.to_uniform(time, self.height as f32, DEFAULT_FOV, false);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Ray Tracing Encoder"),
        });

        // Run compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Ray Tracing Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);

            let workgroup_count_x = (self.width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            let workgroup_count_y = (self.height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
        }

        // Copy texture to staging buffer
        encoder.copy_texture_to_buffer(
            self.output_texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.width),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        // Read pixels (BLOCKING)
        let pixels = gpu.read_buffer_sync(&self.staging_buffer)?;

        Ok(pixels)
    }
}

/// Ray tracing layer logic
#[derive(Clone)]
pub struct RayTracingLogic {
    gpu: Arc<GpuContext>,
    compute: Arc<ComputeState>,
    camera: CameraState,
    scene_name: String,
    elapsed_time: f32,
}

impl RayTracingLogic {
    async fn new(
        gpu: Arc<GpuContext>,
        scene_name: String,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let camera = CameraState::new_for_scene(&scene_name);
        let compute = ComputeState::new(&gpu, &scene_name, width, height).await?;

        Ok(Self {
            gpu,
            compute: Arc::new(compute),
            camera,
            scene_name,
            elapsed_time: 0.0,
        })
    }
}

impl LayerLogic for RayTracingLogic {
    fn update(&self, delta: f32, controller: &dyn Controller) -> Self {
        let new_camera = self.camera.update(delta, controller);

        Self {
            gpu: self.gpu.clone(),
            compute: self.compute.clone(),
            camera: new_camera,
            scene_name: self.scene_name.clone(),
            elapsed_time: self.elapsed_time + delta,
        }
    }

    fn render(&self, _mask: &[bool], _context: &DisplayContext) -> LayerOutput {
        match self.compute.render(&self.gpu, &self.camera, self.elapsed_time) {
            Ok(pixels) => LayerOutput::opaque(pixels),
            Err(e) => {
                eprintln!("Ray tracing render error: {}", e);
                // Return black pixels on error
                let size = (self.compute.width * self.compute.height * 4) as usize;
                LayerOutput::opaque(vec![0; size])
            }
        }
    }
}

/// Builder for ray tracing layer
pub struct RayTracingLayerBuilder {
    gpu: Arc<GpuContext>,
    scene_name: String,
    width: u32,
    height: u32,
    fps: f32,
    priority: i32,
}

impl RayTracingLayerBuilder {
    pub fn new(gpu: Arc<GpuContext>, scene_name: &str, width: u32, height: u32) -> Self {
        Self {
            gpu,
            scene_name: scene_name.to_string(),
            width,
            height,
            fps: 60.0,
            priority: 0,
        }
    }

    pub fn fps(mut self, fps: f32) -> Self {
        self.fps = fps;
        self
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub async fn build(self) -> Result<Box<dyn Layer>> {
        let logic = RayTracingLogic::new(
            self.gpu,
            self.scene_name,
            self.width,
            self.height,
        )
        .await?;

        let layer = TimedLayer::new(logic, self.fps, self.priority);

        Ok(Box::new(layer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_state_creation() {
        let camera = CameraState::new_for_scene("pyramid");
        assert_eq!(camera.position, Vec3::new(0.0, 8.0, 20.0));
    }

    #[test]
    fn test_camera_forward_vector() {
        let camera = CameraState {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
        };

        let forward = camera.forward();
        assert!((forward.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_camera_functional_update() {
        struct MockController;
        impl Controller for MockController {
            fn is_down(&self, _button: Button) -> bool {
                false
            }
            fn get_down_keys(&self) -> &[Button] {
                &[]
            }
        }

        let camera = CameraState::new_for_scene("pyramid");
        let controller = MockController;

        let new_camera = camera.update(0.016, &controller);

        // Position should not change with no input
        assert_eq!(new_camera.position, camera.position);
    }
}
