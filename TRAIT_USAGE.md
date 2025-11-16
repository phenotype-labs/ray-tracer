# Trait Architecture Usage

## Core Abstractions

The ray tracer uses functional trait abstractions for clean separation of concerns:

### 1. WindowContext
Handles display and window management:
```rust
pub trait WindowContext {
    fn dimensions(&self) -> WindowDimensions;
    fn draw(&self, pixels: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn request_redraw(&self);
}
```

### 2. PipelineExecutor
Orchestrates the rendering pipeline:
```rust
pub trait PipelineExecutor {
    fn register_window_dimensions(&mut self, dimensions: WindowDimensions);
    fn frames(&mut self) -> Box<dyn Iterator<Item = FrameData> + '_>;
}
```

### 3. App
Application lifecycle and logic:
```rust
pub trait App {
    fn register<E: PipelineExecutor>(&mut self, executor: &mut E);
    fn update(&mut self, delta_time: f32);
}
```

## Supporting Abstractions

### Frame Timing
Pure timing data provider:
```rust
pub trait FrameTiming {
    fn frame_number(&self) -> u64;
    fn elapsed_time(&self) -> f32;
    fn delta_time(&self) -> f32;
}
```

### Camera Traits
Separated into three concerns:

**CameraTransform** - Immutable spatial state:
```rust
pub trait CameraTransform {
    fn position(&self) -> [f32; 3];
    fn forward(&self) -> [f32; 3];
    fn up(&self) -> [f32; 3];
    fn right(&self) -> [f32; 3];
}
```

**CameraProjection** - Matrix generation:
```rust
pub trait CameraProjection {
    fn view_matrix(&self) -> [[f32; 4]; 4];
    fn projection_matrix(&self, aspect_ratio: f32) -> [[f32; 4]; 4];
}
```

**CameraController** - State updates:
```rust
pub trait CameraController {
    fn update(&mut self, delta_time: f32);
}
```

### Scene Traits
Separated by responsibility:

**SceneGeometry** - Pure geometry access:
```rust
pub trait SceneGeometry {
    fn geometry(&self) -> &[TriangleData];
    fn name(&self) -> &str;
}
```

**SceneBuilder** - Geometry construction:
```rust
pub trait SceneBuilder {
    fn build(&self) -> Vec<TriangleData>;
}
```

**DynamicScene** - Time-based updates:
```rust
pub trait DynamicScene {
    fn update(&mut self, time: f32);
    fn needs_rebuild(&self) -> bool;
}
```

## Usage Pattern

The clean separation enables this elegant usage pattern:

```rust
// Setup
let window = create_window();
let mut executor = create_executor();
let mut app = create_app();

// Registration phase
executor.register_window_dimensions(window.dimensions());
app.register(&mut executor);

// Render loop
for frame in executor.frames() {
    window.draw(frame.pixels())?;
}
```

## Key Benefits

1. **Separation of Concerns**: Each trait has a single, well-defined responsibility
2. **Composability**: Traits can be mixed and matched as needed
3. **Testability**: Each component can be tested independently
4. **Flexibility**: Different implementations can be swapped without changing consumers
5. **Functional Design**: Immutable data flow, pure functions, minimal mutation
6. **Iterator Pattern**: Frames are lazy-evaluated through clean iterator interface

## Design Principles Applied

- **Single Responsibility**: Each trait does one thing well
- **Interface Segregation**: Small, focused interfaces instead of monolithic ones
- **Immutability**: Prefer immutable queries over mutable state where possible
- **Functional Composition**: Traits compose naturally without tight coupling
- **KISS**: Simple, straightforward abstractions
- **DRY**: Reusable building blocks eliminate duplication
