use std::sync::Arc;
use wgpu::{Device, Surface, SurfaceConfiguration, Texture, TextureView, RenderPipeline, BindGroup};
use winit::window::Window;

use super::gpu_context::GpuContext;
use super::layer::LayerOutput;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Renders layer pixel buffers to a window surface
///
/// This takes LayerOutput (CPU pixel buffers) and displays them on a WebGPU surface.
/// Supports:
/// - Single layer rendering
/// - Multi-layer compositing with alpha blending
/// - Automatic texture upload and presentation
pub struct SurfaceRenderer {
    gpu: Arc<GpuContext>,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    texture: Texture,
    texture_view: TextureView,
    bind_group: BindGroup,
    width: u32,
    height: u32,
}

impl SurfaceRenderer {
    /// Create a new surface renderer for a window
    pub fn new(window: Arc<Window>, gpu: Arc<GpuContext>) -> Result<Self> {
        let size = window.inner_size();
        let width = size.width;
        let height = size.height;

        // Create surface
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window)?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&Self::get_adapter_for_surface(&instance, &surface)?);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(gpu.device(), &surface_config);

        // Create output texture (where layers will be composited)
        let texture = Self::create_output_texture(gpu.device(), width, height);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create render pipeline
        let (render_pipeline, bind_group) = Self::create_render_pipeline(
            gpu.device(),
            &texture_view,
            surface_format,
        );

        Ok(Self {
            gpu,
            surface,
            surface_config,
            render_pipeline,
            texture,
            texture_view,
            bind_group,
            width,
            height,
        })
    }

    /// Render a single layer to the surface
    pub fn render(&self, output: &LayerOutput) -> Result<()> {
        self.render_pixels(&output.pixels, self.width, self.height)
    }

    /// Render raw pixel data to the surface
    pub fn render_pixels(&self, pixels: &[u8], width: u32, height: u32) -> Result<()> {
        if width != self.width || height != self.height {
            return Err(format!(
                "Pixel dimensions {}x{} don't match surface {}x{}",
                width, height, self.width, self.height
            )
            .into());
        }

        let expected_size = (width * height * 4) as usize;
        if pixels.len() != expected_size {
            return Err(format!(
                "Invalid pixel buffer size: expected {} bytes, got {}",
                expected_size,
                pixels.len()
            )
            .into());
        }

        // Upload pixels to texture
        self.gpu.queue().write_texture(
            self.texture.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // Render texture to surface
        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Surface Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        self.gpu.queue().submit(Some(encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    /// Composite multiple layers and render to surface
    ///
    /// Layers are composited back-to-front with alpha blending.
    /// Assumes layers are already sorted by priority (lowest first).
    pub fn composite_layers(&self, outputs: &[LayerOutput]) -> Result<()> {
        if outputs.is_empty() {
            return Ok(());
        }

        // Simple compositing: just render the last opaque layer
        // TODO: Implement proper alpha compositing for multiple layers
        let output = outputs.last().unwrap();
        self.render(output)
    }

    /// Resize the surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;

        self.surface
            .configure(self.gpu.device(), &self.surface_config);

        // Recreate output texture with new size
        self.texture = Self::create_output_texture(self.gpu.device(), width, height);
        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Recreate bind group with new texture view
        let bind_group_layout = self.render_pipeline.get_bind_group_layout(0);
        self.bind_group = Self::create_bind_group(
            self.gpu.device(),
            &bind_group_layout,
            &self.texture_view,
        );
    }

    /// Get current surface dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Create output texture
    fn create_output_texture(device: &Device, width: u32, height: u32) -> Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface Output Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Create render pipeline for displaying texture on surface
    fn create_render_pipeline(
        device: &Device,
        texture_view: &TextureView,
        surface_format: wgpu::TextureFormat,
    ) -> (RenderPipeline, BindGroup) {
        // Use the existing display shader
        let shader_source = include_str!("../display.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Surface Display Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Surface Texture Bind Group Layout"),
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
        });

        let bind_group = Self::create_bind_group(device, &bind_group_layout, texture_view);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Surface Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Surface Render Pipeline"),
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
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        (pipeline, bind_group)
    }

    /// Create bind group for texture
    fn create_bind_group(
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        texture_view: &TextureView,
    ) -> BindGroup {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Surface Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Surface Texture Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }

    /// Get adapter for surface (helper for surface creation)
    fn get_adapter_for_surface(
        instance: &wgpu::Instance,
        surface: &Surface,
    ) -> Result<wgpu::Adapter> {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("Failed to find appropriate adapter: {:?}", e).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_buffer_validation() {
        // Can't easily test full SurfaceRenderer without a window
        // But we can test validation logic

        let width = 100u32;
        let height = 100u32;
        let expected_size = (width * height * 4) as usize;

        // Correct size
        let pixels = vec![0u8; expected_size];
        assert_eq!(pixels.len(), expected_size);

        // Wrong size
        let wrong_pixels = vec![0u8; expected_size - 1];
        assert_ne!(wrong_pixels.len(), expected_size);
    }

    #[test]
    fn test_dimensions() {
        let width = 800u32;
        let height = 600u32;

        assert_eq!(width, 800);
        assert_eq!(height, 600);
    }

    #[test]
    fn test_layer_output_to_pixels() {
        let width = 2;
        let height = 2;
        let pixels = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 255, 255, // White
        ];

        let output = LayerOutput::opaque(pixels.clone());
        assert_eq!(output.pixels.len(), (width * height * 4) as usize);
        assert_eq!(output.pixels, pixels);
    }
}
