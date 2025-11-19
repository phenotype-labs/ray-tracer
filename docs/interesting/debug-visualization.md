# Debug & Visualization

debugging, visualization, performance-profiling, validation, development-tools

## Visual Debugging Techniques

### Normal Visualization
Convert normals to RGB for immediate visual feedback.

```rust
fn visualize_normal(normal: Vec3) -> Vec3 {
    // Map [-1, 1] to [0, 1]
    (normal + Vec3::ONE) * 0.5
}
```

**What to look for**:
- **Smooth gradients**: Correctly interpolated normals
- **Sharp discontinuities**: Face normals or geometry edges
- **Unexpected colors**: Flipped or incorrect normals
- **Black/white patches**: Unnormalized vectors

**Color interpretation**:
- Red channel = X axis
- Green channel = Y axis
- Blue channel = Z axis

### Depth Buffer Visualization
```rust
fn visualize_depth(t: f32, near: f32, far: f32) -> f32 {
    // Linear depth
    let linear = (t - near) / (far - near);
    linear.clamp(0.0, 1.0)
}

fn visualize_depth_log(t: f32, near: f32, far: f32) -> f32 {
    // Logarithmic (better perception)
    let linear = (t - near) / (far - near);
    (1.0 + linear).ln() / (1.0 + (far - near)).ln()
}
```

**What to look for**:
- **Expected distance gradation**: Nearer = darker, farther = lighter
- **Sudden jumps**: Missing geometry or intersection bugs
- **Infinite values**: Rays missing all geometry

### Material ID Visualization
Assign unique colors to each material.

```rust
fn visualize_material(material_id: u32) -> Vec3 {
    let hue = (material_id as f32 * 0.618033988749895) % 1.0; // Golden ratio
    hsv_to_rgb(hue, 1.0, 1.0)
}
```

**Use case**: Verify material assignments, debug texture mapping

### UV Coordinate Visualization
```rust
fn visualize_uv(uv: Vec2) -> Vec3 {
    Vec3::new(uv.x, uv.y, 0.0)
}

fn visualize_uv_checkerboard(uv: Vec2, scale: f32) -> f32 {
    let u = (uv.x * scale).floor() as i32;
    let v = (uv.y * scale).floor() as i32;
    if (u + v) % 2 == 0 { 1.0 } else { 0.0 }
}
```

**What to look for**:
- **Smooth gradients**: Correct UV interpolation
- **Seams/discontinuities**: UV unwrapping issues
- **Stretched patterns**: Non-uniform UV scaling

## Acceleration Structure Debugging

### BVH Bounds Visualization
```rust
fn visualize_bvh_bounds(ray: &Ray, scene: &Scene) -> Vec3 {
    let mut depth = 0;
    traverse_bvh(ray, &scene.bvh, &mut depth);

    // Color code by traversal depth
    match depth {
        0 => Vec3::new(0.0, 0.0, 0.0),      // Miss
        1..=3 => Vec3::new(0.0, 1.0, 0.0),   // Efficient (green)
        4..=8 => Vec3::new(1.0, 1.0, 0.0),   // Acceptable (yellow)
        _ => Vec3::new(1.0, 0.0, 0.0),       // Poor (red)
    }
}
```

**Interpretation**:
- **Green**: Good BVH structure, few traversals
- **Yellow**: Acceptable performance
- **Red**: BVH needs optimization (too deep, poor SAH)

### Intersection Count Heatmap
```rust
struct DebugStats {
    ray_tests: u32,
    intersection_tests: u32,
}

fn visualize_intersection_count(stats: &DebugStats) -> Vec3 {
    let normalized = (stats.intersection_tests as f32 / 100.0).min(1.0);

    // Blue (0) -> Green (50) -> Red (100+)
    if normalized < 0.5 {
        Vec3::new(0.0, normalized * 2.0, 1.0 - normalized * 2.0)
    } else {
        Vec3::new((normalized - 0.5) * 2.0, 1.0 - (normalized - 0.5) * 2.0, 0.0)
    }
}
```

**What to look for**:
- **Hot spots (red)**: Complex geometry areas
- **Cold spots (blue)**: Empty space or good culling
- **Uniform distribution**: Well-balanced BVH

## Ray Debugging Modes

### Primary Rays Only
Disable all bounces to isolate basic intersection issues.

```rust
fn debug_primary_only(ray: &Ray) -> Vec3 {
    if let Some(hit) = scene.intersect(ray) {
        hit.material.albedo // No lighting, just base color
    } else {
        Vec3::ZERO
    }
}
```

**Use case**: Verify geometry visibility, test intersection code

### Shadow Ray Visualization
```rust
fn debug_shadow_rays(hit_point: Vec3, lights: &[Light]) -> Vec3 {
    let mut visible_lights = 0;

    for light in lights {
        if !in_shadow(hit_point, light.position) {
            visible_lights += 1;
        }
    }

    // Grayscale: number of visible lights
    Vec3::splat(visible_lights as f32 / lights.len() as f32)
}
```

**Interpretation**:
- **White**: All lights visible
- **Gray**: Partially shadowed
- **Black**: Fully shadowed

### Bounce Depth Visualization
```rust
fn debug_bounce_depth(depth: u32, max_depth: u32) -> Vec3 {
    let t = depth as f32 / max_depth as f32;
    // Gradient from blue (0) to red (max)
    Vec3::new(t, 0.0, 1.0 - t)
}
```

**What to look for**:
- **Mostly blue**: Low bounce counts (efficient)
- **Red patches**: Deep recursion (mirrors, glass)
- **Unexpected patterns**: Potential infinite bounce bugs

## Performance Profiling

### Per-Pixel Timing
```rust
fn profile_pixel_time(x: u32, y: u32) -> Duration {
    let start = Instant::now();
    trace_pixel(x, y);
    start.elapsed()
}

fn visualize_timing_heatmap(timings: &[Duration]) -> Vec3 {
    let max_time = timings.iter().max().unwrap();
    let normalized = timing.as_secs_f32() / max_time.as_secs_f32();

    // Green (fast) -> Yellow -> Red (slow)
    if normalized < 0.5 {
        Vec3::new(normalized * 2.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 1.0 - (normalized - 0.5) * 2.0, 0.0)
    }
}
```

**Hotspot identification**: Red areas need optimization

### Ray Count Statistics
```rust
struct RayStats {
    primary_rays: u64,
    shadow_rays: u64,
    reflection_rays: u64,
    refraction_rays: u64,
}

impl RayStats {
    fn total(&self) -> u64 {
        self.primary_rays + self.shadow_rays + self.reflection_rays + self.refraction_rays
    }

    fn print_breakdown(&self) {
        println!("Primary:    {:>10} ({:>5.1}%)", self.primary_rays,
                 100.0 * self.primary_rays as f64 / self.total() as f64);
        println!("Shadow:     {:>10} ({:>5.1}%)", self.shadow_rays,
                 100.0 * self.shadow_rays as f64 / self.total() as f64);
        println!("Reflection: {:>10} ({:>5.1}%)", self.reflection_rays,
                 100.0 * self.reflection_rays as f64 / self.total() as f64);
        println!("Refraction: {:>10} ({:>5.1}%)", self.refraction_rays,
                 100.0 * self.refraction_rays as f64 / self.total() as f64);
    }
}
```

**Typical breakdown**:
- Primary: 5-10%
- Shadow: 40-60%
- Reflection/Refraction: 30-50%

## Validation & Testing

### NaN/Infinity Detection
```rust
fn validate_color(color: Vec3, x: u32, y: u32) -> Vec3 {
    if !color.x.is_finite() || !color.y.is_finite() || !color.z.is_finite() {
        eprintln!("Invalid color at ({}, {}): {:?}", x, y, color);
        return Vec3::new(1.0, 0.0, 1.0); // Magenta error color
    }
    color
}

fn validate_intersection(t: f32) -> bool {
    t.is_finite() && t >= 0.0
}
```

**NaN sources**:
- Division by zero
- Square root of negative numbers
- Unnormalized vectors in dot products

**Debugging strategy**: Magenta pixels indicate validation failures

### Wireframe Mode
```rust
fn render_wireframe(hit: &Hit) -> bool {
    let bary = hit.barycentric;
    let edge_threshold = 0.02;

    bary.x < edge_threshold || bary.y < edge_threshold || bary.z < edge_threshold
}

fn shade_with_wireframe(hit: &Hit) -> Vec3 {
    if render_wireframe(hit) {
        Vec3::new(0.0, 1.0, 0.0) // Green edges
    } else {
        shade_normal(hit)
    }
}
```

**Use case**: Verify triangle topology, find mesh holes

### Bounding Volume Overlay
```rust
fn render_with_bounds(ray: &Ray, scene: &Scene) -> Vec3 {
    // Check AABB intersection first
    if let Some(aabb_t) = ray.intersect_aabb(&scene.bounds) {
        if let Some(hit) = scene.intersect(ray) {
            if (hit.t - aabb_t).abs() < 0.01 {
                // Hit is on AABB boundary
                return Vec3::new(1.0, 1.0, 0.0); // Yellow wireframe
            }
            return shade(hit);
        }
    }
    environment_color(ray.direction)
}
```

## Debug Output Buffers

### Multi-Buffer Output
```rust
enum DebugBuffer {
    Color,
    Normals,
    Depth,
    MaterialID,
    UV,
    BVHDepth,
    ShadowRays,
}

struct DebugRenderer {
    buffers: HashMap<DebugBuffer, Vec<Vec3>>,
}

impl DebugRenderer {
    fn save_buffer(&self, buffer_type: DebugBuffer, path: &str) {
        let data = &self.buffers[&buffer_type];
        save_image(path, data);
    }

    fn cycle_debug_mode(&mut self) {
        // F-key to cycle through visualization modes
    }
}
```

**Workflow**: Save all debug buffers for offline analysis

### Console Output Sampling
```rust
fn debug_sample_pixels(width: u32, height: u32, samples: u32) {
    let mut rng = thread_rng();

    for _ in 0..samples {
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);

        let ray = camera.generate_ray(x, y);
        let result = trace_ray_debug(ray);

        println!("Pixel ({}, {}): color={:?}, depth={}, bounces={}",
                 x, y, result.color, result.depth, result.bounces);
    }
}
```

**Use case**: Statistical validation without visual inspection

## Common Visual Bugs

### Black Pixels (Self-Intersection)
**Symptom**: Random black pixels despite visible geometry
**Cause**: Shadow rays hitting their own surface
**Fix**: Increase epsilon offset

```rust
const SHADOW_EPSILON: f32 = 0.001; // Too small
const SHADOW_EPSILON: f32 = 0.01;  // Better
```

### Fireflies (Extreme Brightness)
**Symptom**: Bright white pixels scattered randomly
**Cause**: Undersampled specular highlights or caustics
**Fix**: Increase samples, clamp max brightness, or firefly rejection

```rust
fn firefly_clamp(color: Vec3, max_luminance: f32) -> Vec3 {
    let luminance = color.x * 0.2126 + color.y * 0.7152 + color.z * 0.0722;
    if luminance > max_luminance {
        color * (max_luminance / luminance)
    } else {
        color
    }
}
```

### Banding (Insufficient Precision)
**Symptom**: Visible color steps instead of smooth gradients
**Cause**: 8-bit output from HDR accumulation
**Fix**: Dithering or 16-bit output

```rust
fn dither(color: Vec3, x: u32, y: u32) -> Vec3 {
    let noise = (((x * 1664525 + y) ^ (y * 1013904223)) & 0xFF) as f32 / 255.0;
    let dither = (noise - 0.5) / 256.0;
    color + Vec3::splat(dither)
}
```

## Facts

### Visual Debug Outputs
- Normal visualization: map XYZ [-1,1] to RGB [0,1] via `(normal + 1) * 0.5`
- Depth buffer: clamp to [near, far] and normalize for grayscale output
- Material ID: use golden ratio hue distribution for distinct colors
- UV coords: checkerboard pattern reveals stretching and seams

### BVH Performance Debugging
- Traversal depth 1-3 = efficient (green), 4-8 = acceptable (yellow), 9+ = poor (red)
- Hot pixels (many intersection tests) indicate BVH imbalance
- Compare SAH cost before/after BVH optimization
- Visualize AABB bounds to verify tight fitting

### Common Bug Patterns
- Black pixels: self-intersection (increase epsilon from 0.001 to 0.01+)
- Fireflies: extreme brightness from undersampled caustics (clamp or increase samples)
- Magenta pixels: NaN/infinity from division by zero or invalid math
- Banding: insufficient bit depth (add dithering or use 16-bit output)

### Validation Checks
- Check `is_finite()` on all color values before output
- Verify ray direction is normalized (length ≈ 1.0)
- Validate intersection t values are positive and finite
- Test barycentric coordinates sum to 1.0 for triangles

### Performance Profiling
- Primary rays typically 5-10% of total ray count
- Shadow rays dominate at 40-60% (one per light per hit)
- Reflection/refraction rays 30-50% depending on materials
- Hot pixels take 10-100x longer than average (target for optimization)

### Debug Modes
- Primary-only mode: disables all bounces for basic intersection testing
- Shadow-only mode: shows light visibility as grayscale
- Wireframe mode: renders triangle edges (barycentric < 0.02)
- Bounce depth: color-codes recursion depth from blue (0) to red (max)

## Links
- [Core Concepts](./core-concepts) - Ray intersection fundamentals
- [BVH](./bvh) - Acceleration structure optimization
- [Color & Sampling](./color-sampling) - Color space validation
- [Performance](./performance) - Profiling and optimization strategies
