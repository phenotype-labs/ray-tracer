use super::controller::Controller;
use super::display_context::DisplayContext;

/// Output from a layer's render call - just pixels
#[derive(Debug, Clone)]
pub struct LayerOutput {
    /// RGBA pixel data
    pub pixels: Vec<u8>,
    /// Optional alpha mask (0.0 = transparent, 1.0 = opaque)
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

/// Layer with independent update rate control
pub trait Layer {
    /// Update layer state with delta time
    /// Returns new layer state (functional style)
    fn update(&self, delta: f32, controller: &dyn Controller) -> Box<dyn Layer>;

    /// Render layer pixels
    fn render(&self, mask: &[bool], context: &DisplayContext) -> LayerOutput;

    /// Layer priority for composition (lower = background, higher = foreground)
    fn priority(&self) -> i32 {
        0
    }

    /// Get target update rate (Hz) - for compatibility
    fn target_fps(&self) -> f32 {
        60.0
    }
}

/// Core layer logic - implemented by specific layers
pub trait LayerLogic: Clone {
    /// Update layer with delta time
    fn update(&self, delta: f32, controller: &dyn Controller) -> Self;

    /// Render layer output
    fn render(&self, mask: &[bool], context: &DisplayContext) -> LayerOutput;
}

/// Layer that manages its own update timing with internal timer
pub struct TimedLayer<T: LayerLogic> {
    logic: T,
    timer: super::timer::FixedHz,
    priority: i32,
}

impl<T: LayerLogic> TimedLayer<T> {
    /// Create layer with specific update rate
    pub fn new(logic: T, hz: f32, priority: i32) -> Self {
        Self {
            logic,
            timer: super::timer::FixedHz::new(hz),
            priority,
        }
    }

    /// Get target Hz
    pub fn hz(&self) -> f32 {
        1.0 / self.timer.interval
    }
}

impl<T: LayerLogic + 'static> Layer for TimedLayer<T> {
    fn update(&self, delta: f32, controller: &dyn Controller) -> Box<dyn Layer> {
        let mut new_timer = self.timer;

        // Check if enough time has passed
        let new_logic = if new_timer.tick(delta) {
            self.logic.update(delta, controller)
        } else {
            self.logic.clone()
        };

        Box::new(TimedLayer {
            logic: new_logic,
            timer: new_timer,
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
        self.hz()
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

    /// Add layer and return new stack
    pub fn with_layer(mut self, layer: Box<dyn Layer>) -> Self {
        self.layers.push(layer);
        self.layers.sort_by_key(|l| l.priority());
        self
    }

    /// Update all layers - functional transformation
    pub fn update(&self, delta: f32, controller: &dyn Controller) -> LayerStack {
        LayerStack {
            layers: self
                .layers
                .iter()
                .map(|layer| layer.update(delta, controller))
                .collect(),
        }
    }

    /// Aggregate all layer outputs
    pub fn render<'a>(
        &'a self,
        mask: &'a [bool],
        context: &'a DisplayContext,
    ) -> impl Iterator<Item = LayerOutput> + 'a {
        self.layers.iter().map(move |layer| layer.render(mask, context))
    }
}

impl Default for LayerStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::controller::Button;

    struct MockController;
    impl Controller for MockController {
        fn is_down(&self, _button: Button) -> bool {
            false
        }
        fn get_down_keys(&self) -> &[Button] {
            &[]
        }
    }

    #[derive(Clone)]
    struct TestLogic {
        value: u32,
    }

    impl LayerLogic for TestLogic {
        fn update(&self, _delta: f32, _controller: &dyn Controller) -> Self {
            TestLogic {
                value: self.value + 1,
            }
        }

        fn render(&self, _mask: &[bool], _context: &DisplayContext) -> LayerOutput {
            LayerOutput::opaque(vec![self.value as u8; 4])
        }
    }

    #[test]
    fn timed_layer_throttles_updates() {
        let logic = TestLogic { value: 0 };
        let layer = TimedLayer::new(logic, 60.0, 0);
        let controller = MockController;

        // Small delta - should not update
        let layer = layer.update(0.01, &controller);
        let ctx = DisplayContext::new(1, 1);
        let output = layer.render(&vec![true], &ctx);
        assert_eq!(output.pixels[0], 0); // Not updated

        // Large delta - should update
        let layer = layer.update(0.02, &controller);
        let output = layer.render(&vec![true], &ctx);
        assert_eq!(output.pixels[0], 1); // Updated
    }

    #[test]
    fn layer_stack_updates_all() {
        let logic1 = TestLogic { value: 10 };
        let logic2 = TestLogic { value: 20 };

        let layer1 = Box::new(TimedLayer::new(logic1, 60.0, 0));
        let layer2 = Box::new(TimedLayer::new(logic2, 60.0, 5));

        let stack = LayerStack::new().with_layer(layer1).with_layer(layer2);

        let controller = MockController;
        let updated = stack.update(0.02, &controller);

        let ctx = DisplayContext::new(1, 1);
        let outputs: Vec<_> = updated.render(&vec![true], &ctx).collect();

        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].pixels[0], 11);
        assert_eq!(outputs[1].pixels[0], 21);
    }
}
