# Color & Sampling

color-space, hdr, tone-mapping, antialiasing, monte-carlo, noise-reduction

## Color Space Fundamentals

### Linear vs sRGB
**Critical**: All lighting calculations happen in linear space.

```rust
// WRONG: Lighting in gamma space
let gamma_color = texture.sample(uv); // Already gamma-corrected
let lit = gamma_color * light_intensity; // Incorrect!

// CORRECT: Convert to linear, light, then gamma correct
let linear_color = srgb_to_linear(texture.sample(uv));
let lit = linear_color * light_intensity;
let final_color = linear_to_srgb(lit);
```

### sRGB Conversion
```rust
fn srgb_to_linear(srgb: f32) -> f32 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}
```

**Apply gamma correction ONCE at final output, not per-frame accumulation**

## High Dynamic Range (HDR)

### Why HDR Matters
Scene brightness isn't limited to [0, 1]:
- Sunlight: 10,000+ intensity
- Light bulb: 100 intensity
- Moonlight: 0.001 intensity

**Float buffers allow values > 1.0**, preserving this range for tone mapping.

### HDR Accumulation
```rust
struct FrameBuffer {
    color: Vec<Vec3>, // RGB floats, unbounded range
    samples: u32,
}

impl FrameBuffer {
    fn accumulate(&mut self, pixel_index: usize, color: Vec3) {
        self.color[pixel_index] += color;
        self.samples += 1;
    }

    fn resolve(&self, pixel_index: usize) -> Vec3 {
        self.color[pixel_index] / self.samples as f32
    }
}
```

**Never clamp during accumulation** - kills highlight detail

## Tone Mapping

### The Display Problem
HDR scene [0, ∞] → Display [0, 1]

### Reinhard Tone Mapping
Simple, fast, preserves contrast.

```rust
fn reinhard(hdr: Vec3) -> Vec3 {
    hdr / (Vec3::ONE + hdr)
}

fn reinhard_extended(hdr: Vec3, max_white: f32) -> Vec3 {
    let numerator = hdr * (Vec3::ONE + hdr / (max_white * max_white));
    numerator / (Vec3::ONE + hdr)
}
```

**max_white**: Brightness level that becomes pure white (typically 1.5-4.0)

### ACES Filmic
Industry-standard, cinematic look.

```rust
fn aces_filmic(hdr: Vec3) -> Vec3 {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;

    let x = hdr;
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(Vec3::ZERO, Vec3::ONE)
}
```

**Characteristics**:
- Soft highlights (doesn't blow out whites)
- Preserves color saturation in bright regions
- Toe and shoulder curve mimics film

### Exposure Control
Simulates camera aperture/shutter speed.

```rust
fn apply_exposure(hdr: Vec3, exposure: f32) -> Vec3 {
    hdr * 2.0_f32.powf(exposure)
}

// Usage
let exposed = apply_exposure(hdr_color, camera.exposure);
let ldr = aces_filmic(exposed);
```

**Exposure values**:
- +1.0 = doubles brightness
- -1.0 = halves brightness
- 0.0 = neutral

## Sampling Strategies

### One Ray Per Pixel (Baseline)
```rust
for y in 0..height {
    for x in 0..width {
        let ray = camera.generate_ray(x, y);
        let color = trace_ray(ray);
        framebuffer.set(x, y, color);
    }
}
```

**Result**: Fast but noisy, severe aliasing

### Multiple Samples Per Pixel
```rust
fn render_with_msaa(x: u32, y: u32, samples: u32) -> Vec3 {
    let mut accumulated = Vec3::ZERO;

    for _ in 0..samples {
        let jitter_x = random::<f32>();
        let jitter_y = random::<f32>();

        let ray = camera.generate_ray(
            x as f32 + jitter_x,
            y as f32 + jitter_y
        );

        accumulated += trace_ray(ray);
    }

    accumulated / samples as f32
}
```

**Sample count impact**:
- 1 sample: Fast, noisy, aliased
- 4 samples: 4x slower, reduced aliasing
- 16 samples: 16x slower, smooth edges
- 64+ samples: Production quality

### Jittered Sampling
Random offset within pixel bounds.

```rust
fn jittered_sample(pixel_x: u32, pixel_y: u32) -> (f32, f32) {
    let jitter_x = random::<f32>(); // [0, 1)
    let jitter_y = random::<f32>();

    (pixel_x as f32 + jitter_x, pixel_y as f32 + jitter_y)
}
```

**Effect**: Converts aliasing to noise (easier for eye to ignore)

### Stratified Sampling
Subdivide pixel into grid for uniform coverage.

```rust
fn stratified_samples(pixel_x: u32, pixel_y: u32, sqrt_samples: u32) -> Vec<(f32, f32)> {
    let mut samples = Vec::new();
    let cell_size = 1.0 / sqrt_samples as f32;

    for i in 0..sqrt_samples {
        for j in 0..sqrt_samples {
            let jitter_x = random::<f32>() * cell_size;
            let jitter_y = random::<f32>() * cell_size;

            let x = pixel_x as f32 + (i as f32 + jitter_x) * cell_size;
            let y = pixel_y as f32 + (j as f32 + jitter_y) * cell_size;

            samples.push((x, y));
        }
    }

    samples
}
```

**Result**: Better coverage than pure random, less clumping

### Low-Discrepancy Sequences
Deterministic "random" sequences with better distribution.

**Halton Sequence**:
```rust
fn halton(index: u32, base: u32) -> f32 {
    let mut result = 0.0;
    let mut f = 1.0;
    let mut i = index;

    while i > 0 {
        f /= base as f32;
        result += f * (i % base) as f32;
        i /= base;
    }

    result
}

fn sample_halton_2d(index: u32) -> (f32, f32) {
    (halton(index, 2), halton(index, 3))
}
```

**Advantage**: Converges faster than random sampling (fewer samples needed)

## Noise Reduction Techniques

### Progressive Rendering
```rust
struct ProgressiveRenderer {
    accumulation_buffer: Vec<Vec3>,
    sample_count: u32,
}

impl ProgressiveRenderer {
    fn add_sample(&mut self, x: usize, y: usize, color: Vec3) {
        let index = y * width + x;
        self.accumulation_buffer[index] += color;
    }

    fn get_current_result(&self, x: usize, y: usize) -> Vec3 {
        let index = y * width + x;
        self.accumulation_buffer[index] / self.sample_count as f32
    }
}
```

**Effect**: Image gradually becomes cleaner with more samples

### Importance Sampling
Focus rays where they contribute most.

**Example: Cosine-weighted hemisphere sampling**
```rust
fn cosine_weighted_hemisphere(normal: Vec3) -> Vec3 {
    let r1 = random::<f32>() * 2.0 * PI;
    let r2 = random::<f32>();
    let r2_sqrt = r2.sqrt();

    let tangent = build_tangent(normal);
    let bitangent = normal.cross(tangent);

    tangent * (r1.cos() * r2_sqrt)
        + bitangent * (r1.sin() * r2_sqrt)
        + normal * (1.0 - r2).sqrt()
}
```

**Result**: Fewer samples needed for same quality

### Russian Roulette
Randomly terminate paths to save computation.

```rust
fn trace_path(ray: Ray, depth: u32) -> Vec3 {
    if depth > 3 {
        let survival_probability = 0.8;
        if random::<f32>() > survival_probability {
            return Vec3::ZERO; // Terminate
        }
        // Continue with boosted contribution
        return trace_path_recursive(ray, depth) / survival_probability;
    }

    trace_path_recursive(ray, depth)
}
```

**Unbiased**: Statistically correct despite random termination

## Color Bleeding

### Why It Happens
Light bounces carry surface color.

```rust
fn trace_bounce(ray: Ray, hit: &Hit, depth: u32) -> Vec3 {
    if depth >= max_depth {
        return Vec3::ZERO;
    }

    let bounce_dir = sample_hemisphere(hit.normal);
    let bounce_ray = Ray::new(hit.point + hit.normal * EPSILON, bounce_dir);

    // Recursive bounce carries surface color
    let incoming_light = trace_bounce(bounce_ray, ..., depth + 1);

    // Surface absorbs and reflects based on albedo
    hit.material.albedo * incoming_light * hit.normal.dot(bounce_dir)
}
```

**Effect**: Red wall tints nearby white surfaces reddish

## Facts

### Color Space Operations
- All lighting calculations must occur in linear color space
- sRGB gamma correction: `linear^(1/2.2)` for output
- Apply gamma correction once at final display, never during accumulation
- Typical gamma value: 2.2 (sRGB standard)

### HDR & Tone Mapping
- HDR allows brightness values > 1.0 (sun = 10,000+)
- Never clamp colors during accumulation - destroys highlight detail
- Tone mapping converts unbounded HDR to displayable [0,1] range
- Reinhard: simple and fast, ACES: filmic and industry-standard

### Sampling Efficiency
- 1 sample/pixel: noisy, 4 samples: acceptable, 16+: smooth
- Jittered sampling converts aliasing artifacts into noise
- Stratified sampling prevents sample clumping in pixels
- Low-discrepancy sequences (Halton, Sobol) converge faster than random

### Advanced Sampling
- Importance sampling focuses rays where they contribute most light
- Cosine-weighted hemisphere sampling natural for diffuse surfaces
- Russian roulette randomly terminates paths (unbiased with proper weighting)
- Different random seed per sample prevents pattern artifacts

### Color Bleeding
- Bounced light carries surface color (red wall → nearby surfaces tinted red)
- Each bounce multiplies by surface albedo (energy conservation)
- Metallic surfaces reflect colored light, dielectrics reflect white
- Minimum 2-3 bounces needed to see noticeable color bleeding

### Exposure Control
- Exposure +1.0 doubles brightness, -1.0 halves brightness
- Exposure simulates camera aperture/shutter speed
- Apply exposure before tone mapping
- Typical range: -3.0 to +3.0 stops

## Links
- [Materials & Physics](./materials-physics) - Material albedo and reflection
- [Lighting & Shadows](./lighting-shadows) - Light source contribution
- [Advanced Techniques](./advanced-techniques) - Path tracing and BRDF sampling
- [Debug & Visualization](./debug-visualization) - Visualizing color buffers
