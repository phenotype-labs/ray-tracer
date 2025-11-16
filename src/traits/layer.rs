use super::controller::Controller;
use super::display_context::DisplayContext;
use super::frame::Frame;

/// Output from a layer's render call - just pixels
#[derive(Debug, Clone)]
pub struct LayerOutput {
    /// RGBA pixel data
    pub pixels: Vec<u8>,
    /// Optional alpha mask (0.0 = transparent, 1.0 = opaque)
    /// If None, pixels are fully opaque
    pub alpha: Option<Vec<f32>>,
}

impl LayerOutput {
    /// Create output with fully opaque pixels
    pub fn opaque(pixels: Vec<u8>) -> Self {
        Self { pixels, alpha: None }
    }

    /// Create output with alpha mask
    pub fn with_alpha(pixels: Vec<u8>, alpha: Vec<f32>) -> Self {
        Self {
            pixels,
            alpha: Some(alpha),
        }
    }
}

/// Layer with independent frame rate control
pub trait Layer {
    /// Update layer state - returns new state if update should occur
    /// Layer decides internally if enough time has passed based on target_fps
    fn update(&self, frame: &Frame, controller: &dyn Controller) -> Box<dyn Layer>;

    /// Render layer pixels
    /// - mask: bool array indicating which pixels are visible (true = render, false = skip)
    /// - context: display dimensions and metadata
    fn render(&self, mask: &[bool], context: &DisplayContext) -> LayerOutput;

    /// Layer priority for composition (lower = background, higher = foreground)
    fn priority(&self) -> i32 {
        0
    }

    /// Target update rate for this layer (updates per second)
    fn target_fps(&self) -> f32;
}

/// Core layer logic - implemented by specific layers
pub trait LayerLogic: Clone {
    /// Update layer with delta time since last update
    fn update(&self, delta: f32, controller: &dyn Controller) -> Self;

    /// Render layer output with mask and display context
    fn render(&self, mask: &[bool], context: &DisplayContext) -> LayerOutput;
}

/// Layer that tracks its own update timing
pub struct TimedLayer<T: LayerLogic> {
    logic: T,
    last_update: f32,
    target_fps: f32,
    priority: i32,
}

impl<T: LayerLogic> TimedLayer<T> {
    /// Create a new timed layer with specified FPS and priority
    pub fn new(logic: T, target_fps: f32, priority: i32) -> Self {
        Self {
            logic,
            last_update: 0.0,
            target_fps,
            priority,
        }
    }
}

impl<T: LayerLogic + 'static> Layer for TimedLayer<T> {
    fn update(&self, frame: &Frame, controller: &dyn Controller) -> Box<dyn Layer> {
        let time_since_update = frame.time - self.last_update;
        let update_interval = 1.0 / self.target_fps;

        // Only update if enough time passed
        let (new_logic, new_last_update) = if time_since_update >= update_interval {
            (self.logic.update(time_since_update, controller), frame.time)
        } else {
            (self.logic.clone(), self.last_update)
        };

        Box::new(TimedLayer {
            logic: new_logic,
            last_update: new_last_update,
            target_fps: self.target_fps,
            priority: self.priority,
        })
    }

    fn render(&self, mask: &[bool], context: &DisplayContext) -> LayerOutput {
        self.logic.render(mask, context)
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn target_fps(&self) -> f32 {
        self.target_fps
    }
}

/// Composable layer stack
pub struct LayerStack {
    layers: Vec<Box<dyn Layer>>,
}

impl LayerStack {
    /// Create empty layer stack
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add layer and return new stack (functional composition)
    pub fn with_layer(mut self, layer: Box<dyn Layer>) -> Self {
        self.layers.push(layer);
        self.layers.sort_by_key(|l| l.priority());
        self
    }

    /// Update all layers - pure functional transformation
    pub fn update(&self, frame: &Frame, controller: &dyn Controller) -> LayerStack {
        LayerStack {
            layers: self.layers.iter()
                .map(|layer| layer.update(frame, controller))
                .collect(),
        }
    }

    /// Aggregate all layer outputs - lazy iterator
    pub fn render<'a>(&'a self, mask: &'a [bool], context: &'a DisplayContext) -> impl Iterator<Item = LayerOutput> + 'a {
        self.layers.iter().map(move |layer| layer.render(mask, context))
    }
}

impl Default for LayerStack {
    fn default() -> Self {
        Self::new()
    }
}
