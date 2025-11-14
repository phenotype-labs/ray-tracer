use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use winit::window::Window;
use crate::camera::Camera;
use crate::grid::HierarchicalGrid;
use crate::scene::{create_default_scene, create_fractal_scene, create_walls_scene, create_tunnel_scene, create_reflected_scene};
use crate::types::{RayDebugInfo, DebugParams};

pub const WORKGROUP_SIZE: u32 = 8;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct RayTracer {
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
    current_scene: Arc<Mutex<String>>,
    needs_reload: Arc<Mutex<bool>>,
    show_grid: Arc<Mutex<bool>>,
    debug_params_buffer: wgpu::Buffer,
    debug_info_buffer: wgpu::Buffer,
    debug_info: RayDebugInfo,
    debug_pixel: Option<(u32, u32)>,
    clear_debug_requested: Arc<Mutex<bool>>,
    manual_debug_x: String,
    manual_debug_y: String,
}

impl RayTracer {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
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

        let scene_name = std::env::var("SCENE").unwrap_or_else(|_| "fractal".to_string());
        println!("Loading scene: {}", scene_name);

        let boxes = match scene_name.as_str() {
            "walls" => create_walls_scene(),
            "tunnel" => create_tunnel_scene(),
            "default" => create_default_scene(),
            "reflected" => create_reflected_scene(),
            _ => create_fractal_scene(),
        };
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

        let debug_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Params Buffer"),
            contents: bytemuck::cast_slice(&[DebugParams {
                debug_pixel: [0, 0],
                enabled: 0,
                _pad: 0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let debug_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Info Buffer"),
            contents: bytemuck::cast_slice(&[RayDebugInfo::default()]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        let (compute_pipeline, compute_bind_group) = Self::create_compute_pipeline(
            &device,
            &camera_buffer,
            &grid_meta_buffer,
            &coarse_buffer,
            &fine_buffer,
            &box_buffer,
            &output_texture_view,
            &debug_params_buffer,
            &debug_info_buffer,
        );

        let (render_pipeline, render_bind_group) =
            Self::create_render_pipeline(&device, &output_texture_view, surface_config.format);

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
            current_scene: Arc::new(Mutex::new(scene_name)),
            needs_reload: Arc::new(Mutex::new(false)),
            show_grid: Arc::new(Mutex::new(false)),
            debug_params_buffer,
            debug_info_buffer,
            debug_info: RayDebugInfo::default(),
            debug_pixel: None,
            clear_debug_requested: Arc::new(Mutex::new(false)),
            manual_debug_x: String::new(),
            manual_debug_y: String::new(),
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
        let fov = 0.785398;
        let camera_uniform = camera.to_uniform(0.0, 800.0, fov, false);

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
        debug_params_buffer: &wgpu::Buffer,
        debug_info_buffer: &wgpu::Buffer,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raytracer_grid.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
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
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: debug_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: debug_info_buffer.as_entire_binding(),
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

    pub fn render(
        &mut self,
        camera: &Camera,
        window: &Window,
        fps: f32,
        time: f32,
    ) -> std::result::Result<(), wgpu::SurfaceError> {
        let fov = 0.785398;
        let show_grid = *self.show_grid.lock().unwrap();
        let camera_uniform = camera.to_uniform(time, self.size.height as f32, fov, show_grid);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let debug_params = if let Some((x, y)) = self.debug_pixel {
            DebugParams {
                debug_pixel: [x, y],
                enabled: 1,
                _pad: 0,
            }
        } else {
            DebugParams {
                debug_pixel: [0, 0],
                enabled: 0,
                _pad: 0,
            }
        };
        self.queue.write_buffer(
            &self.debug_params_buffer,
            0,
            bytemuck::cast_slice(&[debug_params]),
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

        if self.debug_pixel.is_some() {
            let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Debug Info Staging Buffer"),
                size: std::mem::size_of::<RayDebugInfo>() as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            encoder.copy_buffer_to_buffer(
                &self.debug_info_buffer,
                0,
                &staging_buffer,
                0,
                std::mem::size_of::<RayDebugInfo>() as u64,
            );

            self.queue.submit(std::iter::once(encoder.finish()));

            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).ok();
            });

            self.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            }).ok();

            rx.recv().ok();
            {
                let data = buffer_slice.get_mapped_range();
                self.debug_info = *bytemuck::from_bytes(&data);
            }
            staging_buffer.unmap();

            encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder 2"),
            });
        }

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

        let raw_input = self.egui_state.take_egui_input(window);
        let current_scene = self.current_scene.clone();
        let needs_reload = self.needs_reload.clone();
        let show_grid = self.show_grid.clone();
        let clear_debug_requested = self.clear_debug_requested.clone();
        let num_boxes = self.num_boxes;
        let resolution = (self.size.width, self.size.height);
        let debug_pixel = self.debug_pixel;
        let debug_info = self.debug_info;

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::Window::new("Debug Info")
                .title_bar(true)
                .resizable(false)
                .fixed_pos(egui::pos2(10.0, 10.0))
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.heading(
                        egui::RichText::new(format!("{:.0} FPS", fps))
                            .size(32.0)
                            .color(egui::Color32::from_rgb(74, 158, 255)),
                    );

                    let frame_time_ms = if fps > 0.0 { 1000.0 / fps } else { 0.0 };
                    ui.label(
                        egui::RichText::new(format!("{:.2} ms", frame_time_ms))
                            .size(14.0)
                            .color(egui::Color32::GRAY),
                    );

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);

                    ui.label(
                        egui::RichText::new("Camera")
                            .size(16.0)
                            .color(egui::Color32::from_rgb(100, 200, 100)),
                    );
                    ui.monospace(format!(
                        "Pos: ({:.2}, {:.2}, {:.2})",
                        camera.position.x, camera.position.y, camera.position.z
                    ));
                    ui.monospace(format!(
                        "Yaw: {:.1}° Pitch: {:.1}°",
                        camera.yaw.to_degrees(),
                        camera.pitch.to_degrees()
                    ));

                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(5.0);

                    ui.label(
                        egui::RichText::new("Scene")
                            .size(16.0)
                            .color(egui::Color32::from_rgb(200, 150, 100)),
                    );
                    ui.monospace(format!("Objects: {}", num_boxes));
                    ui.monospace(format!("Name: {}", current_scene.lock().unwrap()));

                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(5.0);

                    ui.label(
                        egui::RichText::new("Rendering")
                            .size(16.0)
                            .color(egui::Color32::from_rgb(200, 100, 200)),
                    );
                    ui.monospace(format!("Resolution: {}x{}", resolution.0, resolution.1));
                    ui.monospace(format!("Time: {:.2}s", time));
                });

            egui::Window::new("Scene Selector")
                .title_bar(true)
                .resizable(false)
                .fixed_pos(egui::pos2(10.0, 310.0))
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        let mut scene = current_scene.lock().unwrap();
                        let mut changed = false;

                        if ui.button("Fractal Scene").clicked() {
                            *scene = "fractal".to_string();
                            changed = true;
                        }
                        if ui.button("Walls Scene").clicked() {
                            *scene = "walls".to_string();
                            changed = true;
                        }
                        if ui.button("Tunnel Scene").clicked() {
                            *scene = "tunnel".to_string();
                            changed = true;
                        }
                        if ui.button("Default Scene").clicked() {
                            *scene = "default".to_string();
                            changed = true;
                        }

                        if changed {
                            *needs_reload.lock().unwrap() = true;
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);

                        let mut show_grid_val = show_grid.lock().unwrap();
                        ui.checkbox(&mut *show_grid_val, "Show Grid Cells");
                    });
                });

            egui::Window::new("Ray Debugger")
                .title_bar(true)
                .resizable(true)
                .default_pos(egui::pos2(resolution.0 as f32 - 340.0, 10.0))
                .default_width(320.0)
                .show(ctx, |ui| {
                    ui.heading(
                        egui::RichText::new("Ray Debug")
                            .size(18.0)
                            .color(egui::Color32::from_rgb(255, 200, 100)),
                    );
                    ui.add_space(5.0);

                    if let Some((x, y)) = debug_pixel {
                        ui.label(
                            egui::RichText::new(format!("Pixel: ({}, {})", x, y))
                                .size(14.0)
                                .color(egui::Color32::from_rgb(100, 200, 255)),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);

                        ui.label(
                            egui::RichText::new("Ray Origin")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(150, 150, 255)),
                        );
                        ui.monospace(format!(
                            "  ({:.2}, {:.2}, {:.2})",
                            debug_info.ray_origin[0], debug_info.ray_origin[1], debug_info.ray_origin[2]
                        ));

                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new("Ray Direction")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(150, 150, 255)),
                        );
                        ui.monospace(format!(
                            "  ({:.3}, {:.3}, {:.3})",
                            debug_info.ray_direction[0], debug_info.ray_direction[1], debug_info.ray_direction[2]
                        ));

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);

                        if debug_info.hit > 0.5 {
                            ui.label(
                                egui::RichText::new("HIT")
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(100, 255, 100)),
                            );

                            ui.monospace(format!("Distance: {:.2}", debug_info.distance));
                            ui.monospace(format!("Object ID: {:.0}", debug_info.object_id));
                            ui.monospace(format!("Steps: {:.0}", debug_info.num_steps));

                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new("Hit Position")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(150, 150, 255)),
                            );
                            ui.monospace(format!(
                                "  ({:.2}, {:.2}, {:.2})",
                                debug_info.hit_position[0], debug_info.hit_position[1], debug_info.hit_position[2]
                            ));

                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new("Hit Normal")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(150, 150, 255)),
                            );
                            ui.monospace(format!(
                                "  ({:.2}, {:.2}, {:.2})",
                                debug_info.hit_normal[0], debug_info.hit_normal[1], debug_info.hit_normal[2]
                            ));

                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new("Surface Color")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(150, 150, 255)),
                            );
                            ui.monospace(format!(
                                "  ({:.2}, {:.2}, {:.2})",
                                debug_info.hit_color[0], debug_info.hit_color[1], debug_info.hit_color[2]
                            ));
                        } else {
                            ui.label(
                                egui::RichText::new("MISS")
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(255, 100, 100)),
                            );
                            ui.monospace(format!("Steps: {:.0}", debug_info.num_steps));
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);

                        if ui.button("Clear Debug Pixel").clicked() {
                            *clear_debug_requested.lock().unwrap() = true;
                        }
                    } else {
                        ui.label("Click on a pixel to debug its ray");
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);

                        ui.label(
                            egui::RichText::new("Manual Entry")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(150, 150, 255)),
                        );
                        ui.label("Enter pixel coordinates:");
                        ui.add_space(5.0);
                        ui.label("(Coming soon)");
                    }
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

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

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

        if *self.clear_debug_requested.lock().unwrap() {
            self.debug_pixel = None;
            *self.clear_debug_requested.lock().unwrap() = false;
            println!("Debug pixel cleared");
        }

        Ok(())
    }

    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        self.egui_state.on_window_event(window, event).consumed
    }

    pub fn needs_reload(&self) -> bool {
        *self.needs_reload.lock().unwrap()
    }

    pub fn get_current_scene(&self) -> String {
        self.current_scene.lock().unwrap().clone()
    }

    pub fn set_debug_pixel(&mut self, x: u32, y: u32) {
        self.debug_pixel = Some((x, y));
        println!("Debug pixel set to ({}, {})", x, y);
    }
}
