# Lighting & Shadows

lighting-models, shadow-rays, physically-based-rendering, light-transport

## Shadow Ray Fundamentals

### Shadow Ray Construction
Shadow rays differ from primary rays:
- **Origin**: Hit point + epsilon offset along normal
- **Direction**: Normalized vector toward light source
- **Purpose**: Boolean visibility test (hit/no hit)
- **Early exit**: Stop at first intersection (no closest-hit needed)

```rust
fn compute_shadow(hit_point: Vec3, surface_normal: Vec3, light_pos: Vec3) -> bool {
    let to_light = (light_pos - hit_point).normalize();
    let shadow_origin = hit_point + surface_normal * SHADOW_EPSILON;
    let shadow_ray = Ray::new(shadow_origin, to_light);

    !scene_intersects_any(shadow_ray, (light_pos - hit_point).length())
}
```

### Shadow Acne Prevention
**Problem**: Ray intersects its own origin surface
**Solution**: Epsilon offset along normal

```rust
const SHADOW_EPSILON: f32 = 0.001; // Tune based on scene scale
let shadow_origin = hit_point + normal * SHADOW_EPSILON;
```

**Critical**: Epsilon too small = shadow acne, epsilon too large = detached shadows

## Light Types

### Point Lights
- **Single shadow ray** per light
- **Inverse square falloff**: `intensity / distance²`
- **Position in world space**: No transformation needed per-frame
- **Hard shadows**: Binary visibility

```rust
fn point_light_contribution(hit_point: Vec3, light: &PointLight) -> f32 {
    let to_light = light.position - hit_point;
    let distance = to_light.length();
    let attenuation = 1.0 / (distance * distance);

    if compute_shadow(hit_point, normal, light.position) {
        return 0.0;
    }

    light.intensity * attenuation
}
```

### Directional Lights
- **No distance falloff**: Simulates sun at infinity
- **Constant direction**: All rays parallel
- **Fastest to compute**: No distance calculations
- **Shadow rays have no t_max**: Trace to scene bounds

```rust
fn directional_light(normal: Vec3, light_dir: Vec3) -> f32 {
    normal.dot(-light_dir).max(0.0)
}
```

### Area Lights
- **Multiple shadow rays**: Sample light surface
- **Soft shadows**: Penumbra effect
- **More expensive**: N samples per light
- **Importance sampling**: Focus samples on visible portion

Minimum 16 samples for acceptable soft shadows, 64+ for production quality.

## Lighting Models

### Lambertian Diffuse
Perfectly diffuse surface - light scatters equally in all directions.

```rust
fn lambertian(normal: Vec3, light_dir: Vec3, albedo: Vec3) -> Vec3 {
    let n_dot_l = normal.dot(light_dir).max(0.0);
    albedo * n_dot_l / PI
}
```

**Division by π**: Energy conservation for physically-based rendering

### Phong Specular
Simple approximation for shiny surfaces.

```rust
fn phong_specular(view_dir: Vec3, reflect_dir: Vec3, shininess: f32) -> f32 {
    let r_dot_v = reflect_dir.dot(view_dir).max(0.0);
    r_dot_v.powf(shininess)
}
```

**Shininess**: 2-10 (rough), 100-1000 (mirror-like)

### Cook-Torrance (PBR)
Industry-standard microfacet model.

```rust
fn cook_torrance(normal: Vec3, view: Vec3, light: Vec3, roughness: f32, metallic: f32) -> Vec3 {
    let halfway = (view + light).normalize();

    let D = ggx_distribution(normal, halfway, roughness);
    let F = fresnel_schlick(view, halfway, metallic);
    let G = smith_geometry(normal, view, light, roughness);

    (D * F * G) / (4.0 * normal.dot(view) * normal.dot(light))
}
```

## Shadow Techniques

### Hard Shadows (Point/Directional)
```rust
fn trace_shadow(origin: Vec3, direction: Vec3, max_distance: f32) -> bool {
    scene.intersects_any(origin, direction, 0.001, max_distance)
}
```

**Performance**: Single ray per light = fast
**Quality**: Unrealistic binary shadows

### Soft Shadows (Area Lights)
```rust
fn soft_shadow(hit_point: Vec3, area_light: &AreaLight, samples: u32) -> f32 {
    let mut visibility = 0.0;

    for _ in 0..samples {
        let light_sample = area_light.sample_point();
        if !trace_shadow(hit_point, light_sample - hit_point, ...) {
            visibility += 1.0;
        }
    }

    visibility / samples as f32
}
```

**Performance**: N rays per light = expensive
**Quality**: Realistic penumbra

### Ambient Occlusion
Approximates indirect lighting by testing hemisphere visibility.

```rust
fn ambient_occlusion(hit_point: Vec3, normal: Vec3, samples: u32) -> f32 {
    let mut occlusion = 0.0;

    for _ in 0..samples {
        let random_dir = cosine_weighted_hemisphere(normal);
        if !scene.intersects_any(hit_point, random_dir, 0.001, AO_DISTANCE) {
            occlusion += 1.0;
        }
    }

    occlusion / samples as f32
}
```

**Typical AO distance**: 1-5 scene units
**Minimum samples**: 8-16 for acceptable quality

## Optimization Strategies

### Shadow Ray Early Exit
```rust
fn any_intersection(ray: &Ray, max_t: f32) -> bool {
    for object in scene {
        if object.intersects(ray, 0.001, max_t) {
            return true; // Early exit!
        }
    }
    false
}
```

**Impact**: 2-5x faster than closest-hit for shadows

### Light Culling
Don't test lights that can't contribute:
- Behind surface (normal.dot(to_light) < 0)
- Too far (distance > light.radius)
- Occluded by previous frame (temporal coherence)

### Shadow Map Hybrid
- Rasterize shadow maps for primary lights
- Ray trace only for secondary lighting
- 10-100x faster for scenes with few moving lights

## Facts

### Shadow Ray Properties
- Shadow rays start from hit point + epsilon offset to prevent self-intersection
- Typical epsilon values: 0.001 for world-space scenes, adjust for scale
- Shadow rays only need boolean result, not closest intersection
- Point lights require one shadow ray, area lights need 16-64 samples

### Light Calculations
- Inverse square law: `intensity / (distance * distance)`
- Directional lights have no distance falloff
- Check surface visibility before lighting calculation
- Store light positions in world space for efficiency

### Performance Characteristics
- Early exit on first shadow ray hit saves 2-5x computation
- Sort objects by distance for shadow rays (closer = more likely to occlude)
- Area light soft shadows cost 16-64x more than hard shadows
- Ambient occlusion requires 8-16 hemisphere samples minimum

### Lighting Models
- Lambertian diffuse divides by π for energy conservation
- Fresnel effect increases reflection at grazing angles
- Cook-Torrance microfacet model is industry standard for PBR
- Metallic materials reflect colored light, dielectrics reflect white

## Links
- [Materials & Physics](./materials-physics) - Material properties and BRDFs
- [Core Concepts](./core-concepts) - Ray fundamentals
- [Color & Sampling](./color-sampling) - HDR and tone mapping
