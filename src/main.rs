use glam::Vec3;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// === Constants ===

const WORKGROUP_SIZE: u32 = 8;
const CAMERA_SPEED: f32 = 0.1;
const FPS_UPDATE_INTERVAL: f32 = 1.0;
const INITIAL_WINDOW_WIDTH: u32 = 800;
const INITIAL_WINDOW_HEIGHT: u32 = 600;

// === Type Aliases ===

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// === GPU Data Structures ===

/// Camera uniform buffer data for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    position: [f32; 3],
    _pad1: f32,
    forward: [f32; 3],
    _pad2: f32,
    right: [f32; 3],
    _pad3: f32,
    up: [f32; 3],
    _pad4: f32,
}

/// Box primitive data for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BoxData {
    min: [f32; 3],
    _pad1: f32,
    max: [f32; 3],
    _pad2: f32,
    color: [f32; 3],
    _pad3: f32,
}

impl BoxData {
    const fn new(min: [f32; 3], max: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            min,
            _pad1: 0.0,
            max,
            _pad2: 0.0,
            color,
            _pad3: 0.0,
        }
    }

    fn bounds(&self) -> AABB {
        AABB {
            min: Vec3::from_array(self.min),
            max: Vec3::from_array(self.max),
        }
    }
}

/// Axis-Aligned Bounding Box
#[derive(Copy, Clone, Debug)]
struct AABB {
    min: Vec3,
    max: Vec3,
}

impl AABB {
    fn union(&self, other: &AABB) -> AABB {
        AABB {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }
}

// === Hierarchical Grid System ===

const GRID_LEVELS: usize = 4;
const FINEST_CELL_SIZE: f32 = 16.0;  // Smallest cells: 16x16x16 units
const MAX_OBJECTS_PER_CELL: usize = 64;

/// Grid metadata for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GridMetadata {
    bounds_min: [f32; 3],
    num_levels: u32,
    bounds_max: [f32; 3],
    finest_cell_size: f32,
    grid_sizes: [[u32; 4]; GRID_LEVELS],  // Size of each level (padded to vec4)
}

/// Fine grid cell data for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FineCellData {
    object_indices: [u32; MAX_OBJECTS_PER_CELL],
    count: u32,
    _pad: [u32; 3],
}

/// Coarse grid level (stores only object count)
struct CoarseGridLevel {
    cell_size: f32,
    grid_size: [usize; 3],
    counts: Vec<u8>,  // Flattened 3D array of object counts
}

impl CoarseGridLevel {
    fn new(bounds: &AABB, cell_size: f32) -> Self {
        let extent = bounds.max - bounds.min;
        let grid_size = [
            (extent.x / cell_size).ceil() as usize + 1,
            (extent.y / cell_size).ceil() as usize + 1,
            (extent.z / cell_size).ceil() as usize + 1,
        ];

        let total_cells = grid_size[0] * grid_size[1] * grid_size[2];

        Self {
            cell_size,
            grid_size,
            counts: vec![0; total_cells],
        }
    }

    fn cell_index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.grid_size[0] + z * self.grid_size[0] * self.grid_size[1]
    }

    fn increment_cell(&mut self, x: usize, y: usize, z: usize) {
        let idx = self.cell_index(x, y, z);
        if self.counts[idx] < 255 {
            self.counts[idx] += 1;
        }
    }
}

/// Finest grid level (stores actual object indices)
struct FineGridLevel {
    cell_size: f32,
    grid_size: [usize; 3],
    cells: Vec<Vec<u32>>,  // Each cell contains object indices
}

impl FineGridLevel {
    fn new(bounds: &AABB, cell_size: f32) -> Self {
        let extent = bounds.max - bounds.min;
        let grid_size = [
            (extent.x / cell_size).ceil() as usize + 1,
            (extent.y / cell_size).ceil() as usize + 1,
            (extent.z / cell_size).ceil() as usize + 1,
        ];

        let total_cells = grid_size[0] * grid_size[1] * grid_size[2];

        Self {
            cell_size,
            grid_size,
            cells: vec![Vec::new(); total_cells],
        }
    }

    fn cell_index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.grid_size[0] + z * self.grid_size[0] * self.grid_size[1]
    }

    fn add_object(&mut self, x: usize, y: usize, z: usize, object_id: u32) {
        let idx = self.cell_index(x, y, z);
        if self.cells[idx].len() < MAX_OBJECTS_PER_CELL {
            self.cells[idx].push(object_id);
        }
    }
}

/// Complete hierarchical grid structure
struct HierarchicalGrid {
    bounds: AABB,
    coarse_levels: Vec<CoarseGridLevel>,
    fine_level: FineGridLevel,
}

impl HierarchicalGrid {
    fn build(objects: &[BoxData]) -> Self {
        // Calculate scene bounds
        let mut bounds = objects[0].bounds();
        for obj in &objects[1..] {
            bounds = bounds.union(&obj.bounds());
        }

        // Add padding
        let padding = Vec3::splat(1.0);
        bounds.min -= padding;
        bounds.max += padding;

        println!("Grid bounds: {:?} to {:?}", bounds.min, bounds.max);

        // Create levels (from coarse to fine)
        let mut coarse_levels = Vec::new();
        for level in 0..(GRID_LEVELS - 1) {
            let cell_size = FINEST_CELL_SIZE * (1 << (GRID_LEVELS - 1 - level)) as f32;
            coarse_levels.push(CoarseGridLevel::new(&bounds, cell_size));
            println!("Coarse level {}: {}x{}x{} cells (size: {})",
                level,
                coarse_levels[level].grid_size[0],
                coarse_levels[level].grid_size[1],
                coarse_levels[level].grid_size[2],
                cell_size);
        }

        let fine_level = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);
        println!("Fine level: {}x{}x{} cells (size: {})",
            fine_level.grid_size[0],
            fine_level.grid_size[1],
            fine_level.grid_size[2],
            FINEST_CELL_SIZE);

        let mut grid = Self {
            bounds,
            coarse_levels,
            fine_level,
        };

        // Assign objects to grid
        for (obj_id, obj) in objects.iter().enumerate() {
            grid.assign_object(obj, obj_id as u32);
        }

        // Print statistics
        let total_coarse_cells: usize = grid.coarse_levels.iter()
            .map(|level| level.counts.len())
            .sum();
        let occupied_fine_cells = grid.fine_level.cells.iter()
            .filter(|cell| !cell.is_empty())
            .count();

        println!("Grid stats:");
        println!("  Coarse cells total: {}", total_coarse_cells);
        println!("  Fine cells occupied: {}/{}",
            occupied_fine_cells,
            grid.fine_level.cells.len());

        grid
    }

    fn assign_object(&mut self, obj: &BoxData, obj_id: u32) {
        let obj_min = Vec3::from_array(obj.min);
        let obj_max = Vec3::from_array(obj.max);
        let bounds_min = self.bounds.min;

        // Assign to all coarse levels
        for level in self.coarse_levels.iter_mut() {
            let cell_size = level.cell_size;
            let grid_size = level.grid_size;

            let min_cell = Self::world_to_cell_static(&obj_min, bounds_min, cell_size);
            let max_cell = Self::world_to_cell_static(&obj_max, bounds_min, cell_size);

            for x in min_cell.x..=max_cell.x {
                for y in min_cell.y..=max_cell.y {
                    for z in min_cell.z..=max_cell.z {
                        let xu = x as usize;
                        let yu = y as usize;
                        let zu = z as usize;
                        if xu < grid_size[0] && yu < grid_size[1] && zu < grid_size[2] {
                            level.increment_cell(xu, yu, zu);
                        }
                    }
                }
            }
        }

        // Assign to fine level
        let cell_size = self.fine_level.cell_size;
        let grid_size = self.fine_level.grid_size;

        let min_cell = Self::world_to_cell_static(&obj_min, bounds_min, cell_size);
        let max_cell = Self::world_to_cell_static(&obj_max, bounds_min, cell_size);

        for x in min_cell.x..=max_cell.x {
            for y in min_cell.y..=max_cell.y {
                for z in min_cell.z..=max_cell.z {
                    let xu = x as usize;
                    let yu = y as usize;
                    let zu = z as usize;
                    if xu < grid_size[0] &&
                       yu < grid_size[1] &&
                       zu < grid_size[2] {
                        self.fine_level.add_object(xu, yu, zu, obj_id);
                    }
                }
            }
        }
    }

    fn world_to_cell_static(pos: &Vec3, bounds_min: Vec3, cell_size: f32) -> glam::UVec3 {
        let rel_pos = *pos - bounds_min;
        glam::UVec3::new(
            (rel_pos.x / cell_size).floor().max(0.0) as u32,
            (rel_pos.y / cell_size).floor().max(0.0) as u32,
            (rel_pos.z / cell_size).floor().max(0.0) as u32,
        )
    }

    /// Flatten grid to GPU-compatible buffers
    fn to_gpu_buffers(&self) -> (GridMetadata, Vec<u8>, Vec<FineCellData>) {
        // Create metadata (pad vec3 to vec4 for WGSL alignment)
        let mut grid_sizes = [[0u32; 4]; GRID_LEVELS];
        for (i, level) in self.coarse_levels.iter().enumerate() {
            grid_sizes[i] = [
                level.grid_size[0] as u32,
                level.grid_size[1] as u32,
                level.grid_size[2] as u32,
                0, // Padding
            ];
        }
        grid_sizes[GRID_LEVELS - 1] = [
            self.fine_level.grid_size[0] as u32,
            self.fine_level.grid_size[1] as u32,
            self.fine_level.grid_size[2] as u32,
            0, // Padding
        ];

        let metadata = GridMetadata {
            bounds_min: self.bounds.min.to_array(),
            num_levels: GRID_LEVELS as u32,
            bounds_max: self.bounds.max.to_array(),
            finest_cell_size: FINEST_CELL_SIZE,
            grid_sizes,
        };

        // Flatten coarse level counts
        let mut all_counts = Vec::new();
        for level in &self.coarse_levels {
            all_counts.extend_from_slice(&level.counts);
        }

        // Flatten fine level cells
        let fine_cells: Vec<FineCellData> = self.fine_level.cells.iter().map(|cell| {
            let mut cell_data = FineCellData {
                object_indices: [0; MAX_OBJECTS_PER_CELL],
                count: cell.len() as u32,
                _pad: [0; 3],
            };
            for (i, &obj_id) in cell.iter().take(MAX_OBJECTS_PER_CELL).enumerate() {
                cell_data.object_indices[i] = obj_id;
            }
            cell_data
        }).collect();

        (metadata, all_counts, fine_cells)
    }
}

// === Camera System ===

/// Movement direction flags
#[derive(Default, Clone, Copy)]
struct MovementState {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl MovementState {
    const fn velocity(&self) -> (f32, f32, f32) {
        let forward = self.forward as i32 - self.backward as i32;
        let right = self.right as i32 - self.left as i32;
        let up = self.up as i32 - self.down as i32;

        (forward as f32, right as f32, up as f32)
    }
}

/// Camera with first-person controls
struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    movement: MovementState,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 8.0, 15.0),  // Higher and further back to see the whole scene
            yaw: std::f32::consts::PI,  // Look towards negative Z (into the scene)
            pitch: -0.6,  // Look down at the scene
            movement: MovementState::default(),
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    fn up(&self) -> Vec3 {
        Vec3::Y
    }

    fn update(&mut self) {
        let (fwd, right_dir, up_dir) = self.movement.velocity();

        let forward_vec = self.forward() * fwd * CAMERA_SPEED;
        let right_vec = self.right() * right_dir * CAMERA_SPEED;
        let up_vec = Vec3::Y * up_dir * CAMERA_SPEED;

        self.position += forward_vec + right_vec + up_vec;
    }

    fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            position: self.position.to_array(),
            _pad1: 0.0,
            forward: self.forward().to_array(),
            _pad2: 0.0,
            right: self.right().to_array(),
            _pad3: 0.0,
            up: self.up().to_array(),
            _pad4: 0.0,
        }
    }

    fn process_keyboard(&mut self, event: &KeyEvent) {
        let is_pressed = event.state.is_pressed();
        if let PhysicalKey::Code(keycode) = event.physical_key {
            match keycode {
                KeyCode::KeyW => self.movement.forward = is_pressed,
                KeyCode::KeyS => self.movement.backward = is_pressed,
                KeyCode::KeyA => self.movement.left = is_pressed,
                KeyCode::KeyD => self.movement.right = is_pressed,
                KeyCode::Space => self.movement.up = is_pressed,
                KeyCode::ShiftLeft => self.movement.down = is_pressed,
                _ => {}
            }
        }
    }
}

// === Scene Configuration ===

/// Creates a huge scene to stress test the BVH
fn create_default_scene() -> Vec<BoxData> {
    let mut boxes = vec![
        // Ground plane
        BoxData::new([-50.0, -1.0, -50.0], [50.0, -0.99, 50.0], [0.3, 0.3, 0.3]),
    ];

    // Dense grid of cubes (20x20 = 400 boxes)
    for x in -10..10 {
        for z in -10..10 {
            let fx = x as f32 * 1.5;
            let fz = z as f32 * 1.5 - 15.0;
            let size = 0.4;

            let color = [
                ((x + 10) as f32 / 20.0) * 0.8 + 0.2,
                ((z + 10) as f32 / 20.0) * 0.8 + 0.2,
                0.6,
            ];

            boxes.push(BoxData::new(
                [fx - size, -0.5, fz - size],
                [fx + size, 0.5, fz + size],
                color,
            ));
        }
    }

    // Floating structures above (15x15x3 = 675 boxes)
    for x in -7..8 {
        for z in -7..8 {
            for y in 0..3 {
                let fx = x as f32 * 2.0;
                let fy = y as f32 * 2.0 + 2.0;
                let fz = z as f32 * 2.0 - 10.0;
                let size = 0.35;

                let color = [
                    ((x + 7) as f32 / 15.0) * 0.7 + 0.3,
                    ((y) as f32 / 3.0) * 0.5 + 0.4,
                    ((z + 7) as f32 / 15.0) * 0.7 + 0.3,
                ];

                boxes.push(BoxData::new(
                    [fx - size, fy - size, fz - size],
                    [fx + size, fy + size, fz + size],
                    color,
                ));
            }
        }
    }

    // Scattered random boxes (200 boxes)
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let hasher_builder = RandomState::new();
    for i in 0..200 {
        let mut hasher = hasher_builder.build_hasher();
        i.hash(&mut hasher);
        let hash = hasher.finish();

        let x = ((hash % 100) as f32 / 100.0) * 40.0 - 20.0;
        let y = (((hash >> 8) % 100) as f32 / 100.0) * 8.0 - 2.0;
        let z = (((hash >> 16) % 100) as f32 / 100.0) * 40.0 - 30.0;
        let size = (((hash >> 24) % 50) as f32 / 100.0) * 0.4 + 0.2;

        let color = [
            ((hash % 100) as f32 / 100.0) * 0.8 + 0.2,
            (((hash >> 8) % 100) as f32 / 100.0) * 0.8 + 0.2,
            (((hash >> 16) % 100) as f32 / 100.0) * 0.8 + 0.2,
        ];

        boxes.push(BoxData::new(
            [x - size, y - size, z - size],
            [x + size, y + size, z + size],
            color,
        ));
    }

    // Tall pillars on the sides (8x10 = 80 boxes)
    for side in [-15.0, 15.0] {
        for z in -5..5 {
            for y in 0..10 {
                let fz = z as f32 * 2.0 - 10.0;
                let fy = y as f32 * 1.5;
                let size = 0.5;

                let color = if side < 0.0 {
                    [0.8, 0.3, 0.3]
                } else {
                    [0.3, 0.3, 0.8]
                };

                boxes.push(BoxData::new(
                    [side - size, fy - size, fz - size],
                    [side + size, fy + size, fz + size],
                    color,
                ));
            }
        }
    }

    println!("Scene created: {} boxes", boxes.len());
    boxes
}

// === Rendering System ===

/// GPU-accelerated ray tracer using compute shaders
struct RayTracer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    size: winit::dpi::PhysicalSize<u32>,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
    num_boxes: usize,
}

impl RayTracer {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;
        let adapter = Self::request_adapter(&instance, &surface).await?;
        let (device, queue) = Self::request_device(&adapter).await?;

        let surface_config = Self::create_surface_config(&surface, &adapter, size);
        surface.configure(&device, &surface_config);

        // Build hierarchical grid from scene
        let boxes = create_default_scene();
        let num_boxes = boxes.len();

        println!("Building Hierarchical Grid...");
        let grid = HierarchicalGrid::build(&boxes);
        let (metadata, coarse_counts, fine_cells) = grid.to_gpu_buffers();

        let grid_meta_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Metadata"),
            contents: bytemuck::cast_slice(&[metadata]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let coarse_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Coarse Counts"),
            contents: &coarse_counts,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let fine_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fine Cells"),
            contents: bytemuck::cast_slice(&fine_cells),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let box_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Box Buffer"),
            contents: bytemuck::cast_slice(&boxes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let camera_buffer = Self::create_camera_buffer(&device);
        let (_output_texture, output_texture_view) = Self::create_output_texture(&device, size);

        let (compute_pipeline, compute_bind_group) = Self::create_compute_pipeline(
            &device,
            &camera_buffer,
            &grid_meta_buffer,
            &coarse_buffer,
            &fine_buffer,
            &box_buffer,
            &output_texture_view,
        );

        let (render_pipeline, render_bind_group) =
            Self::create_render_pipeline(&device, &output_texture_view, surface_config.format);

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_config.format,
            egui_wgpu::RendererOptions::default(),
        );

        println!("Ray tracer initialized: {} boxes", num_boxes);

        Ok(Self {
            device,
            queue,
            surface,
            size,
            compute_pipeline,
            compute_bind_group,
            camera_buffer,
            render_pipeline,
            render_bind_group,
            egui_renderer,
            egui_state,
            egui_ctx,
            num_boxes,
        })
    }

    async fn request_adapter(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
    ) -> Result<wgpu::Adapter> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| "Failed to find appropriate adapter".into())
    }

    async fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue)> {
        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|e| e.into())
    }

    fn create_surface_config(
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> wgpu::SurfaceConfiguration {
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }

    fn create_camera_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let camera = Camera::new();
        let camera_uniform = camera.to_uniform();

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_output_texture(
        device: &wgpu::Device,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_compute_pipeline(
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        grid_meta_buffer: &wgpu::Buffer,
        coarse_buffer: &wgpu::Buffer,
        fine_buffer: &wgpu::Buffer,
        box_buffer: &wgpu::Buffer,
        output_texture_view: &wgpu::TextureView,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raytracer_grid.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Binding 0: Camera
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
                // Binding 1: Grid Metadata
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: Coarse level counts
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
                // Binding 3: Fine level cells
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
                // Binding 4: Boxes
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
                // Binding 5: Output texture
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
            label: Some("grid_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: grid_meta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: coarse_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: fine_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: box_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(output_texture_view),
                },
            ],
            label: Some("grid_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        (pipeline, bind_group)
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        output_texture_view: &wgpu::TextureView,
        surface_format: wgpu::TextureFormat,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Display Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("display.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("render_bind_group_layout"),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(output_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("render_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Display Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        (pipeline, bind_group)
    }

    fn render(
        &mut self,
        camera: &Camera,
        window: &Window,
        fps: f32,
    ) -> std::result::Result<(), wgpu::SurfaceError> {
        let camera_uniform = camera.to_uniform();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // Compute pass - ray tracing
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            let workgroup_size_x = self.size.width.div_ceil(WORKGROUP_SIZE);
            let workgroup_size_y = self.size.height.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroup_size_x, workgroup_size_y, 1);
        }

        // Render pass - display ray traced image
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Display Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        // egui pass - UI overlay
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::Window::new("FPS")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(egui::pos2(10.0, 10.0))
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!("{:.0}", fps))
                            .size(48.0)
                            .color(egui::Color32::from_rgb(74, 158, 255)),
                    );
                    ui.label(
                        egui::RichText::new("FPS")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                });
        });

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .egui_ctx
            .tessellate(full_output.shapes, self.egui_ctx.pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        // Update egui buffers
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        // Render egui - using scoped render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // SAFETY: The render pass lifetime is actually tied to the encoder,
            // but egui-wgpu requires 'static. This is safe because we drop the
            // render pass before using the encoder again.
            let render_pass_static = unsafe {
                std::mem::transmute::<&mut wgpu::RenderPass<'_>, &mut wgpu::RenderPass<'static>>(
                    &mut render_pass,
                )
            };

            self.egui_renderer
                .render(render_pass_static, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        self.egui_state.on_window_event(window, event).consumed
    }
}

// === Application ===

struct App {
    window: Option<Arc<Window>>,
    raytracer: Option<RayTracer>,
    camera: Camera,
    last_frame_time: Instant,
    frame_count: u32,
    fps: f32,
    fps_update_timer: f32,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            raytracer: None,
            camera: Camera::new(),
            last_frame_time: Instant::now(),
            frame_count: 0,
            fps: 0.0,
            fps_update_timer: 0.0,
        }
    }

    fn update_fps(&mut self, delta: f32) {
        self.frame_count += 1;
        self.fps_update_timer += delta;

        if self.fps_update_timer >= FPS_UPDATE_INTERVAL {
            self.fps = self.frame_count as f32 / self.fps_update_timer;
            println!("FPS: {:.1}", self.fps);
            self.frame_count = 0;
            self.fps_update_timer = 0.0;
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = match event_loop.create_window(
                Window::default_attributes()
                    .with_title("Ray Tracer")
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        INITIAL_WINDOW_WIDTH,
                        INITIAL_WINDOW_HEIGHT,
                    )),
            ) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    eprintln!("Failed to create window: {}", e);
                    event_loop.exit();
                    return;
                }
            };

            let raytracer = match pollster::block_on(RayTracer::new(window.clone())) {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to initialize ray tracer: {}", e);
                    event_loop.exit();
                    return;
                }
            };

            self.window = Some(window);
            self.raytracer = Some(raytracer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle the event first
        if let (Some(raytracer), Some(window)) = (&mut self.raytracer, &self.window) {
            if raytracer.handle_event(window, &event) {
                return; // egui consumed the event
            }
        }

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => self.camera.process_keyboard(&event),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta = now.duration_since(self.last_frame_time).as_secs_f32();
                self.last_frame_time = now;

                self.update_fps(delta);
                self.camera.update();

                if let (Some(raytracer), Some(window)) = (&mut self.raytracer, &self.window) {
                    if let Err(e) = raytracer.render(&self.camera, window, self.fps) {
                        eprintln!("Render error: {}", e);
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    let mut app = App::new();

    println!("Ray Tracer - Controls: WASD, Space/Shift, Escape to quit");
    event_loop.run_app(&mut app)?;

    Ok(())
}
