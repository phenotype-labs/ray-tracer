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
            position: Vec3::new(0.0, 2.0, 5.0),
            yaw: std::f32::consts::PI,
            pitch: -0.3,
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

/// Creates the default scene with boxes
fn create_default_scene() -> Vec<BoxData> {
    vec![
        BoxData::new([-1.5, -0.5, -3.0], [-0.5, 0.5, -2.0], [1.0, 0.2, 0.2]),
        BoxData::new([0.5, -0.5, -4.0], [1.5, 0.5, -3.0], [0.2, 1.0, 0.2]),
        BoxData::new([-0.5, -0.5, -6.0], [0.5, 0.5, -5.0], [0.2, 0.2, 1.0]),
        BoxData::new([-10.0, -1.0, -20.0], [10.0, -0.99, 0.0], [0.5, 0.5, 0.5]),
    ]
}

// === Rendering System ===

/// GPU-accelerated ray tracer using compute shaders
struct RayTracer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    _config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    _output_texture: wgpu::Texture,
    _output_texture_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
}

impl RayTracer {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;
        let adapter = Self::request_adapter(&instance, &surface).await?;
        let (device, queue) = Self::request_device(&adapter).await?;

        let config = Self::create_surface_config(&surface, &adapter, size);
        surface.configure(&device, &config);

        let boxes = create_default_scene();
        let box_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Box Buffer"),
            contents: bytemuck::cast_slice(&boxes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let camera_buffer = Self::create_camera_buffer(&device);
        let (output_texture, output_texture_view) = Self::create_output_texture(&device, size);

        let (compute_pipeline, compute_bind_group) = Self::create_compute_pipeline(
            &device,
            &camera_buffer,
            &box_buffer,
            &output_texture_view,
        );

        let (render_pipeline, render_bind_group) =
            Self::create_render_pipeline(&device, &output_texture_view, config.format);

        println!("Ray tracer initialized: {} boxes", boxes.len());

        Ok(Self {
            device,
            queue,
            surface,
            _config: config,
            size,
            compute_pipeline,
            compute_bind_group,
            camera_buffer,
            _output_texture: output_texture,
            _output_texture_view: output_texture_view,
            render_pipeline,
            render_bind_group,
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
            .ok_or("Failed to find appropriate adapter".into())
    }

    async fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue)> {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
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
        box_buffer: &wgpu::Buffer,
        output_texture_view: &wgpu::TextureView,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raytracer.wgsl").into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
                label: Some("compute_bind_group_layout"),
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
                    resource: box_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(output_texture_view),
                },
            ],
            label: Some("compute_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
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

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
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

    fn render(&mut self, camera: &Camera) -> std::result::Result<(), wgpu::SurfaceError> {
        let camera_uniform = camera.to_uniform();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

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

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

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
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
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

                if let Some(raytracer) = &mut self.raytracer {
                    if let Err(e) = raytracer.render(&self.camera) {
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

    println!("Simple Ray Tracer - Controls: WASD, Space/Shift, Escape to quit");
    event_loop.run_app(&mut app)?;

    Ok(())
}
