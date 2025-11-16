use crate::core::{Controller, DisplayContext, Frame, RenderPipeline, WindowContext, WindowRenderer};

/// Display combines rendering pipeline and window output
pub struct Display<P: RenderPipeline, W: WindowRenderer> {
    pipeline: P,
    renderer: W,
}

impl<P: RenderPipeline, W: WindowRenderer> Display<P, W> {
    /// Create new display with pipeline and renderer
    pub fn new(pipeline: P, renderer: W) -> Self {
        Self { pipeline, renderer }
    }

    /// Full render cycle: update layers → render → display
    pub fn draw(&mut self, frame: &Frame, controller: &dyn Controller, context: &DisplayContext)
        -> Result<(), Box<dyn std::error::Error>>
    {
        // Update all layers at their respective rates
        self.pipeline.update(frame, controller);

        // Render composed scene
        let pixels = self.pipeline.render(context);

        // Display to window
        self.renderer.render(&pixels)
    }

    /// Register window with renderer
    pub fn register_window(&mut self, window: &dyn WindowContext) {
        self.renderer.register_window(window);
    }

    /// Access pipeline for configuration
    pub fn pipeline(&self) -> &P {
        &self.pipeline
    }

    /// Access mutable pipeline
    pub fn pipeline_mut(&mut self) -> &mut P {
        &mut self.pipeline
    }

    /// Access renderer
    pub fn renderer(&self) -> &W {
        &self.renderer
    }

    /// Access mutable renderer
    pub fn renderer_mut(&mut self) -> &mut W {
        &mut self.renderer
    }
}
