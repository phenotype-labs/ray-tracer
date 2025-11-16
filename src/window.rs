use std::sync::Arc;
use winit::window::Window as WinitWindow;
use crate::camera::Camera;
use crate::renderer::RayTracer;
use crate::frame::FrameInfo;

/// Wrapper around winit Window with imperative draw API
pub struct Window {
    inner: Arc<WinitWindow>,
}

impl Window {
    pub fn new(window: Arc<WinitWindow>) -> Self {
        Self { inner: window }
    }

    pub fn inner(&self) -> &Arc<WinitWindow> {
        &self.inner
    }

    /// Draw a frame using the provided renderer, camera, and frame info
    pub fn draw(
        &self,
        raytracer: &mut RayTracer,
        camera: &Camera,
        fps: f32,
        frame: &FrameInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        raytracer.render(camera, &self.inner, fps, frame.time, frame.number)?;
        Ok(())
    }

    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    pub fn inner_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.inner.inner_size()
    }
}
