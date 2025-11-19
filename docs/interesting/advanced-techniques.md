# Advanced Techniques

path-tracing, global-illumination, importance-sampling, next-event-estimation, photon-mapping, real-time-ray-tracing

## Path Tracing Fundamentals

### What is Path Tracing?
Ray tracing with random bounces to simulate global illumination.

**Key difference from basic ray tracing**:
- **Ray tracing**: Deterministic reflection/refraction
- **Path tracing**: Random sampling of BRDF for full light transport

```rust
fn path_trace(ray: Ray, depth: u32) -> Vec3 {
    if depth >= MAX_DEPTH {
        return Vec3::ZERO;
    }

    let Some(hit) = scene.intersect(ray) else {
        return environment(ray.direction);
    };

    // Emission (light sources)
    let emitted = hit.material.emissive;

    // Random bounce direction based on BRDF
    let bounce_dir = sample_brdf(hit.normal, ray.direction, &hit.material);
    let bounce_ray = Ray::new(hit.point + hit.normal * EPSILON, bounce_dir);

    // Recursive path trace
    let incoming = path_trace(bounce_ray, depth + 1);

    // BRDF evaluation
    let brdf = evaluate_brdf(hit.normal, ray.direction, bounce_dir, &hit.material);
    let cos_theta = hit.normal.dot(bounce_dir).max(0.0);

    emitted + brdf * incoming * cos_theta
}
```

### Monte Carlo Integration
Path tracing solves the rendering equation via Monte Carlo sampling.

**Rendering equation**:
```
L_o(x, ω_o) = L_e(x, ω_o) + ∫ f_r(x, ω_i, ω_o) L_i(x, ω_i) cos(θ_i) dω_i
```

Where:
- `L_o`: Outgoing radiance
- `L_e`: Emitted radiance
- `f_r`: BRDF (Bidirectional Reflectance Distribution Function)
- `L_i`: Incoming radiance
- `cos(θ_i)`: Lambert's cosine law

**Monte Carlo estimator**:
```rust
fn monte_carlo_estimate(samples: u32) -> Vec3 {
    let mut sum = Vec3::ZERO;

    for _ in 0..samples {
        let direction = sample_hemisphere(normal);
        let pdf = probability_density(direction);
        let radiance = trace_ray(direction);

        sum += radiance / pdf;
    }

    sum / samples as f32
}
```

## Next Event Estimation (Direct Light Sampling)

### The Problem with Pure Path Tracing
Random bounces rarely hit light sources directly → high noise.

### The Solution
Explicitly sample lights at each bounce.

```rust
fn next_event_estimation(hit: &Hit, view_dir: Vec3) -> Vec3 {
    let mut direct_light = Vec3::ZERO;

    // Sample all lights directly
    for light in scene.lights {
        let to_light = (light.position - hit.point).normalize();

        // Shadow ray
        if !in_shadow(hit.point, light.position) {
            let brdf = evaluate_brdf(hit.normal, view_dir, to_light, &hit.material);
            let cos_theta = hit.normal.dot(to_light).max(0.0);
            let distance = (light.position - hit.point).length();
            let attenuation = 1.0 / (distance * distance);

            direct_light += brdf * light.intensity * attenuation * cos_theta;
        }
    }

    // Indirect lighting via random bounce
    let bounce_dir = sample_brdf(hit.normal, view_dir, &hit.material);
    let indirect = path_trace(Ray::new(hit.point, bounce_dir), ...);

    direct_light + indirect
}
```

**Performance gain**: 10-100x noise reduction compared to pure path tracing

## Importance Sampling

### Why Uniform Sampling is Wasteful
Most directions contribute little light, but we sample them equally.

### Cosine-Weighted Hemisphere Sampling
Prefer directions aligned with surface normal.

```rust
fn cosine_weighted_sample(normal: Vec3) -> (Vec3, f32) {
    let r1 = random::<f32>() * 2.0 * PI;
    let r2 = random::<f32>();
    let r2_sqrt = r2.sqrt();

    // Local coordinates
    let x = r1.cos() * r2_sqrt;
    let y = r1.sin() * r2_sqrt;
    let z = (1.0 - r2).sqrt();

    // Transform to world space
    let (tangent, bitangent) = build_tangent_space(normal);
    let direction = tangent * x + bitangent * y + normal * z;

    let pdf = z / PI; // cos(θ) / π

    (direction, pdf)
}
```

**PDF (Probability Density Function)**: `cos(θ) / π`

### GGX Importance Sampling (Specular)
Sample microfacet distribution for shiny surfaces.

```rust
fn sample_ggx(normal: Vec3, roughness: f32) -> (Vec3, f32) {
    let r1 = random::<f32>() * 2.0 * PI;
    let r2 = random::<f32>();

    let a = roughness * roughness;
    let theta = ((a * a * r2) / (1.0 - r2)).sqrt().atan();
    let phi = r1;

    // Spherical to Cartesian
    let x = theta.sin() * phi.cos();
    let y = theta.sin() * phi.sin();
    let z = theta.cos();

    let (tangent, bitangent) = build_tangent_space(normal);
    let halfway = tangent * x + bitangent * y + normal * z;

    let pdf = ggx_pdf(normal, halfway, roughness);

    (halfway, pdf)
}
```

**Result**: Sharp highlights converge with fewer samples

## Russian Roulette Path Termination

### The Problem
Fixed max depth wastes computation on dark paths.

### Probabilistic Termination
```rust
fn russian_roulette(ray: Ray, throughput: Vec3, depth: u32) -> Vec3 {
    if depth < MIN_DEPTH {
        return trace_path(ray, throughput, depth);
    }

    // Survival probability based on path brightness
    let p_survival = throughput.max_component().min(0.95);

    if random::<f32>() < p_survival {
        // Continue with boosted contribution
        trace_path(ray, throughput / p_survival, depth)
    } else {
        // Terminate
        Vec3::ZERO
    }
}
```

**Properties**:
- Unbiased (statistically correct)
- Early terminates dim paths
- Boosts surviving paths to compensate

**Typical values**:
- `MIN_DEPTH`: 3-5 bounces
- `p_survival`: Based on path throughput (0.5-0.95)

## Bidirectional Path Tracing (BDPT)

### Concept
Trace paths from both camera AND lights, then connect them.

```rust
fn bidirectional_path_trace() -> Vec3 {
    // Camera path
    let camera_path = trace_from_camera(camera_ray);

    // Light path
    let light_path = trace_from_light(random_light);

    // Connect all pairs
    let mut radiance = Vec3::ZERO;
    for (cam_vertex, light_vertex) in camera_path.iter().cartesian_product(&light_path) {
        radiance += connect_vertices(cam_vertex, light_vertex);
    }

    radiance / (camera_path.len() * light_path.len()) as f32
}
```

**Advantages**:
- Excellent for caustics (light paths hit specular surfaces)
- Better for indoor scenes (light paths find geometry)

**Disadvantages**:
- Complex implementation
- 2-5x slower than unidirectional

## Photon Mapping

### Two-Pass Algorithm

**Pass 1: Photon Emission**
```rust
fn emit_photons(num_photons: u32) {
    let mut photon_map = Vec::new();

    for _ in 0..num_photons {
        let light = sample_random_light();
        let direction = sample_light_direction(light);
        let mut photon = Photon {
            position: light.position,
            direction,
            power: light.emission / num_photons as f32,
        };

        // Trace photon through scene
        trace_photon(&mut photon, &mut photon_map);
    }

    build_kdtree(photon_map)
}

fn trace_photon(photon: &mut Photon, map: &mut Vec<Photon>) {
    for depth in 0..MAX_DEPTH {
        let Some(hit) = scene.intersect(photon.direction) else { break; };

        // Store on diffuse surfaces
        if hit.material.is_diffuse() {
            map.push(photon.clone());
        }

        // Russian roulette bounce
        if !russian_roulette_continue(photon.power) {
            break;
        }

        // Bounce photon
        photon.direction = sample_brdf(hit.normal, photon.direction, &hit.material);
        photon.position = hit.point;
        photon.power *= hit.material.albedo;
    }
}
```

**Pass 2: Rendering with Photon Map**
```rust
fn radiance_estimate(point: Vec3, normal: Vec3, photon_map: &KdTree) -> Vec3 {
    let k_nearest = photon_map.query_k_nearest(point, 100);
    let max_distance = k_nearest.last().unwrap().distance;

    let mut irradiance = Vec3::ZERO;
    for photon in k_nearest {
        irradiance += photon.power;
    }

    // Density estimation
    irradiance / (PI * max_distance * max_distance)
}
```

**Use cases**:
- Caustics (focused light through glass)
- Indirect illumination caching
- Participating media (fog, smoke)

## Real-Time Ray Tracing Hybrid

### Rasterization + Ray Tracing
Combine techniques for best of both worlds.

```rust
fn hybrid_render() -> Vec3 {
    // Rasterize primary visibility
    let gbuffer = rasterize_scene();

    // Ray trace from G-buffer
    for pixel in gbuffer {
        // Shadows via ray tracing
        let shadow = trace_shadow_ray(pixel.position, light.position);

        // Reflections via ray tracing
        let reflection = if pixel.material.roughness < 0.3 {
            trace_reflection_ray(pixel.position, pixel.normal)
        } else {
            sample_reflection_probe(pixel.position)
        };

        // Combine
        pixel.color = shade(gbuffer, shadow, reflection);
    }
}
```

**Performance**:
- Primary rays: ~0 cost (rasterization)
- Shadow rays: 1-4 per pixel
- Reflection rays: Only for shiny surfaces

**Frame time**: 16ms @ 1080p (60 FPS achievable)

### Temporal Accumulation
Reuse previous frames to reduce per-frame sample count.

```rust
fn temporal_accumulate(current_frame: Vec3, prev_frame: Vec3, alpha: f32) -> Vec3 {
    // Blend with history
    current_frame * alpha + prev_frame * (1.0 - alpha)
}
```

**Typical alpha**: 0.05-0.2 (accumulates over 5-20 frames)

**Challenges**:
- Motion reprojection
- Disocclusion handling
- Ghosting artifacts

### Denoising
AI-based denoisers (DLSS, OptiX) reduce required samples.

**Traditional path tracing**: 1024+ samples per pixel
**With AI denoiser**: 1-4 samples per pixel

**Input buffers**:
- Noisy color
- Normals
- Depth
- Motion vectors

**Output**: Clean, denoised image

## Volumetric Rendering

### Ray Marching Through Volume
```rust
fn trace_volume(ray: Ray) -> Vec3 {
    let mut transmittance = 1.0;
    let mut scattered_light = Vec3::ZERO;

    let step_size = 0.1;
    let mut t = 0.0;

    while t < ray.t_max {
        let pos = ray.at(t);
        let density = volume_density(pos);

        // Absorption
        transmittance *= (-density * step_size).exp();

        // In-scattering
        let light = sample_volume_lighting(pos);
        scattered_light += transmittance * light * density * step_size;

        t += step_size;
    }

    scattered_light
}
```

**Applications**: Fog, clouds, smoke, underwater

## Facts

### Path Tracing
- Path tracing = ray tracing with random BRDF sampling for global illumination
- Monte Carlo integration solves rendering equation statistically
- Typical convergence: 64-1024 samples per pixel for clean images
- Each bounce multiplies by BRDF × cos(θ) and divides by PDF

### Next Event Estimation
- Direct light sampling reduces noise by 10-100x vs pure path tracing
- Sample lights explicitly at each bounce instead of hoping to hit them randomly
- Combine direct lighting (NEE) with indirect lighting (random bounces)
- Critical for scenes with small light sources

### Importance Sampling
- Cosine-weighted hemisphere sampling: PDF = `cos(θ) / π`
- GGX sampling focuses on specular highlights for rough surfaces
- Match sampling distribution to BRDF for variance reduction
- Importance sampling reduces required samples by 5-10x

### Russian Roulette
- Probabilistically terminate paths after minimum depth (3-5 bounces)
- Survival probability based on path throughput (brightness)
- Unbiased: boost surviving paths by `1 / p_survival`
- Saves 20-50% computation by terminating dark paths early

### Advanced Techniques
- Bidirectional path tracing: trace from both camera and lights, then connect
- Photon mapping: cache indirect lighting in spatial structure (kd-tree)
- BDPT excellent for caustics, 2-5x slower than unidirectional
- Photon maps use k-nearest density estimation for irradiance

### Real-Time Optimization
- Hybrid rendering: rasterize primary rays, ray trace shadows/reflections
- Temporal accumulation: blend with previous frames (alpha 0.05-0.2)
- AI denoisers reduce required samples from 1024+ to 1-4
- Target: 60 FPS with 1-4 samples per pixel + denoising

### Volumetric Rendering
- Ray marching with fixed step size (0.1-1.0 scene units)
- Transmittance decreases exponentially: `e^(-density × distance)`
- In-scattering accumulates light along ray path
- Applications: fog, clouds, atmospheric scattering

## Links
- [Core Concepts](./core-concepts) - Ray fundamentals
- [Materials & Physics](./materials-physics) - BRDF evaluation
- [Lighting & Shadows](./lighting-shadows) - Direct illumination
- [Color & Sampling](./color-sampling) - Monte Carlo sampling techniques
- [Performance](./performance) - Optimization strategies
