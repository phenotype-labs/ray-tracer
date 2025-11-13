use egui_wgpu::ScreenDescriptor;
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

struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 5.0),
            yaw: std::f32::consts::PI,
            pitch: -0.3,
            speed: 0.1,
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
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
        let forward = self.forward();
        let right = self.right();

        if self.move_forward {
            self.position += forward * self.speed;
        }
        if self.move_backward {
            self.position -= forward * self.speed;
        }
        if self.move_right {
            self.position += right * self.speed;
        }
        if self.move_left {
            self.position -= right * self.speed;
        }
        if self.move_up {
            self.position.y += self.speed;
        }
        if self.move_down {
            self.position.y -= self.speed;
        }
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
                KeyCode::KeyW => self.move_forward = is_pressed,
                KeyCode::KeyS => self.move_backward = is_pressed,
                KeyCode::KeyA => self.move_left = is_pressed,
                KeyCode::KeyD => self.move_right = is_pressed,
                KeyCode::Space => self.move_up = is_pressed,
                KeyCode::ShiftLeft => self.move_down = is_pressed,
                _ => {}
            }
        }
    }
}

struct RayTracer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    output_texture: wgpu::Texture,
    output_texture_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    egui_renderer: egui_wgpu::Renderer,
}

impl RayTracer {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
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
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let boxes = vec![
            BoxData {
                min: [-1.5, -0.5, -3.0],
                _pad1: 0.0,
                max: [-0.5, 0.5, -2.0],
                _pad2: 0.0,
                color: [1.0, 0.2, 0.2],
                _pad3: 0.0,
            },
            BoxData {
                min: [0.5, -0.5, -4.0],
                _pad1: 0.0,
                max: [1.5, 0.5, -3.0],
                _pad2: 0.0,
                color: [0.2, 1.0, 0.2],
                _pad3: 0.0,
            },
            BoxData {
                min: [-0.5, -0.5, -6.0],
                _pad1: 0.0,
                max: [0.5, 0.5, -5.0],
                _pad2: 0.0,
                color: [0.2, 0.2, 1.0],
                _pad3: 0.0,
            },
            BoxData {
                min: [-10.0, -1.0, -20.0],
                _pad1: 0.0,
                max: [10.0, -0.99, 0.0],
                _pad2: 0.0,
                color: [0.5, 0.5, 0.5],
                _pad3: 0.0,
            },
        ];
        let box_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Box Buffer"),
            contents: bytemuck::cast_slice(&boxes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let camera = Camera::new();
        let camera_uniform = camera.to_uniform();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
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
        let output_texture_view =
            output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raytracer.wgsl").into()),
        });

        let compute_bind_group_layout =
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

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
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
                    resource: wgpu::BindingResource::TextureView(&output_texture_view),
                },
            ],
            label: Some("compute_bind_group"),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        let display_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Display Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("display.wgsl").into()),
        });

        let render_bind_group_layout =
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

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("render_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Display Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &display_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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

        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);

        println!("Ray tracer initialized: 4 boxes");

        Self {
            device,
            queue,
            surface,
            config,
            size,
            compute_pipeline,
            compute_bind_group,
            camera_buffer,
            output_texture,
            output_texture_view,
            render_pipeline,
            render_bind_group,
            egui_renderer,
        }
    }

    fn render(
        &mut self,
        camera: &Camera,
        window: &Window,
        egui_state: &mut egui_winit::State,
        egui_ctx: &egui::Context,
        fps: f32,
    ) -> Result<(), wgpu::SurfaceError> {
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
            let workgroup_size_x = (self.size.width + 7) / 8;
            let workgroup_size_y = (self.size.height + 7) / 8;
            compute_pass.dispatch_workgroups(workgroup_size_x, workgroup_size_y, 1);
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let raw_input = egui_state.take_egui_input(window);
        let full_output = egui_ctx.run(raw_input, |ctx| {
            egui::Window::new("Ray Tracer Info")
                .default_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    ui.heading("Performance");
                    ui.label(format!("FPS: {:.1}", fps));
                    ui.separator();

                    ui.heading("Camera");
                    ui.label(format!(
                        "Position: ({:.2}, {:.2}, {:.2})",
                        camera.position.x, camera.position.y, camera.position.z
                    ));
                    ui.label(format!("Yaw: {:.2}", camera.yaw));
                    ui.label(format!("Pitch: {:.2}", camera.pitch));
                    ui.separator();

                    ui.heading("Controls");
                    ui.label("WASD - Move horizontally");
                    ui.label("Space - Move up");
                    ui.label("Shift - Move down");
                    ui.label("ESC - Quit");
                });
        });

        egui_state.handle_platform_output(window, full_output.platform_output);

        let clipped_primitives =
            egui_ctx.tessellate(full_output.shapes, egui_ctx.pixels_per_point());

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

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

        {
            let mut egui_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.egui_renderer.render(
                &mut egui_render_pass,
                &clipped_primitives,
                &screen_descriptor,
            );
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

struct App {
    window: Option<Arc<Window>>,
    raytracer: Option<RayTracer>,
    camera: Camera,
    last_frame_time: Instant,
    frame_count: u32,
    fps: f32,
    fps_update_timer: f32,
    egui_state: Option<egui_winit::State>,
    egui_ctx: egui::Context,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Ray Tracer")
                            .with_inner_size(winit::dpi::LogicalSize::new(800, 600)),
                    )
                    .unwrap(),
            );
            let egui_state = egui_winit::State::new(
                self.egui_ctx.clone(),
                egui::ViewportId::ROOT,
                &window,
                Some(window.scale_factor() as f32),
                None,
                None,
            );
            let raytracer = pollster::block_on(RayTracer::new(window.clone()));
            self.window = Some(window);
            self.raytracer = Some(raytracer);
            self.egui_state = Some(egui_state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(egui_state) = &mut self.egui_state {
            let _ = egui_state.on_window_event(&self.window.as_ref().unwrap(), &event);
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
                self.frame_count += 1;
                self.fps_update_timer += delta;
                if self.fps_update_timer >= 1.0 {
                    self.fps = self.frame_count as f32 / self.fps_update_timer;
                    println!("FPS: {:.1}", self.fps);
                    self.frame_count = 0;
                    self.fps_update_timer = 0.0;
                }
                self.camera.update();
                if let (Some(raytracer), Some(window), Some(egui_state)) =
                    (&mut self.raytracer, &self.window, &mut self.egui_state)
                {
                    let _ = raytracer.render(
                        &self.camera,
                        window,
                        egui_state,
                        &self.egui_ctx,
                        self.fps,
                    );
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

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        window: None,
        raytracer: None,
        camera: Camera::new(),
        last_frame_time: Instant::now(),
        frame_count: 0,
        fps: 0.0,
        fps_update_timer: 0.0,
        egui_state: None,
        egui_ctx: egui::Context::default(),
    };
    println!("Simple Ray Tracer - Controls: WASD, Space/Shift, Escape to quit");
    event_loop.run_app(&mut app).unwrap();
}
