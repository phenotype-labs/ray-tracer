# Materials & Physics

materials, reflection, refraction, transparency, index-of-refraction, physically-based-rendering

## Material System Architecture

### Material Data Structure
```rust
struct Material {
    // Base properties
    albedo: Vec3,           // Base color (0-1 range)
    metallic: f32,          // 0 = dielectric, 1 = metal
    roughness: f32,         // 0 = mirror, 1 = matte

    // Optical properties
    ior: f32,               // Index of refraction (1.0 = air)
    transmission: f32,      // 0 = opaque, 1 = transparent

    // Emission
    emissive: Vec3,         // Self-illumination
    emissive_strength: f32,
}
```

**Store materials with geometry**: Avoids indirection, improves cache locality

## Reflection

### Perfect Mirror Reflection
Angle in = angle out (law of reflection)

```rust
fn reflect(incident: Vec3, normal: Vec3) -> Vec3 {
    incident - 2.0 * incident.dot(normal) * normal
}
```

**Critical**: Both incident and normal must be normalized

### Glossy Reflection
Perfect mirror + random perturbation based on roughness

```rust
fn glossy_reflect(incident: Vec3, normal: Vec3, roughness: f32) -> Vec3 {
    let perfect_reflection = reflect(incident, normal);
    let random_dir = sample_ggx_distribution(normal, roughness);

    // Blend based on roughness
    (perfect_reflection + random_dir * roughness).normalize()
}
```

**Roughness mapping**:
- 0.0 = perfect mirror
- 0.1-0.3 = glossy metal
- 0.5-0.7 = satin/brushed
- 0.9-1.0 = matte/diffuse

### Reflection Depth Limiting
**Problem**: Infinite bounces between parallel mirrors
**Solution**: Maximum recursion depth

```rust
fn trace_reflection(ray: Ray, depth: u32, max_depth: u32) -> Vec3 {
    if depth >= max_depth {
        return Vec3::ZERO; // Terminate recursion
    }

    if let Some(hit) = scene.intersect(ray) {
        let reflected_ray = Ray::new(hit.point, reflect(ray.dir, hit.normal));
        let reflected_color = trace_reflection(reflected_ray, depth + 1, max_depth);

        // Accumulate with energy decay
        hit.material.albedo * reflected_color * 0.9.powi(depth as i32)
    } else {
        environment_color(ray.direction)
    }
}
```

**Typical max depth**: 3-8 bounces
**Energy decay**: Each bounce reduces contribution

## Refraction & Transparency

### Snell's Law
Light bends when entering materials with different IOR.

```rust
fn refract(incident: Vec3, normal: Vec3, eta: f32) -> Option<Vec3> {
    let cos_theta_i = -incident.dot(normal);
    let sin2_theta_i = (1.0 - cos_theta_i * cos_theta_i).max(0.0);
    let sin2_theta_t = eta * eta * sin2_theta_i;

    // Total internal reflection check
    if sin2_theta_t > 1.0 {
        return None;
    }

    let cos_theta_t = (1.0 - sin2_theta_t).sqrt();
    Some(eta * incident + (eta * cos_theta_i - cos_theta_t) * normal)
}
```

### Index of Refraction (IOR) Values
| Material | IOR |
|----------|-----|
| Vacuum | 1.0 |
| Air | 1.000293 (~1.0) |
| Water | 1.333 |
| Glass | 1.5-1.9 |
| Diamond | 2.417 |
| Sapphire | 1.77 |

### Total Internal Reflection
Occurs at steep angles when light travels from dense to less dense medium.

```rust
fn trace_glass(ray: Ray, hit: &Hit) -> Vec3 {
    let (n1, n2, normal) = if ray.inside_object {
        (hit.material.ior, 1.0, -hit.normal) // Exiting
    } else {
        (1.0, hit.material.ior, hit.normal)  // Entering
    };

    let eta = n1 / n2;

    match refract(ray.direction, normal, eta) {
        Some(refracted) => {
            // Refraction
            trace_ray(Ray::new(hit.point + refracted * EPSILON, refracted), ...)
        }
        None => {
            // Total internal reflection
            let reflected = reflect(ray.direction, normal);
            trace_ray(Ray::new(hit.point + reflected * EPSILON, reflected), ...)
        }
    }
}
```

### Glass Material (Reflection + Refraction)
Real glass both reflects and refracts light.

```rust
fn trace_glass_realistic(ray: Ray, hit: &Hit) -> Vec3 {
    let fresnel = schlick_fresnel(ray.direction, hit.normal, hit.material.ior);

    // Split ray into reflection and refraction
    let reflected_color = trace_reflection(ray, hit, ...);
    let refracted_color = trace_refraction(ray, hit, ...);

    // Blend based on Fresnel
    reflected_color * fresnel + refracted_color * (1.0 - fresnel)
}
```

### Beer's Law (Absorption)
Light absorption through transparent media.

```rust
fn apply_beer_law(color: Vec3, distance: f32, absorption: Vec3) -> Vec3 {
    Vec3::new(
        color.x * (-absorption.x * distance).exp(),
        color.y * (-absorption.y * distance).exp(),
        color.z * (-absorption.z * distance).exp(),
    )
}
```

**Use case**: Colored glass, translucent liquids, fog

## Fresnel Effect

### Schlick's Approximation
More reflection at grazing angles.

```rust
fn schlick_fresnel(view: Vec3, normal: Vec3, ior: f32) -> f32 {
    let r0 = ((1.0 - ior) / (1.0 + ior)).powi(2);
    let cos_theta = (-view).dot(normal).max(0.0);
    r0 + (1.0 - r0) * (1.0 - cos_theta).powi(5)
}
```

**Effect**: Looking straight at glass = 4% reflection, grazing angle = 100% reflection

### Metallic vs Dielectric Fresnel
- **Dielectrics (glass, plastic)**: Reflect white light
- **Metals (gold, copper)**: Reflect colored light

```rust
fn fresnel_color(base_reflectivity: Vec3, metallic: f32, view: Vec3, normal: Vec3) -> Vec3 {
    let f0 = Vec3::lerp(Vec3::splat(0.04), base_reflectivity, metallic);
    let cos_theta = (-view).dot(normal).max(0.0);

    f0 + (Vec3::ONE - f0) * (1.0 - cos_theta).powi(5)
}
```

## Material Types

### Diffuse (Lambertian)
```rust
fn sample_diffuse(normal: Vec3) -> Vec3 {
    cosine_weighted_hemisphere(normal)
}
```
**Properties**: Scatters light uniformly, no specular highlights

### Metallic
```rust
fn sample_metallic(incident: Vec3, normal: Vec3, roughness: f32) -> Vec3 {
    let reflected = reflect(incident, normal);
    perturb_by_roughness(reflected, roughness)
}
```
**Properties**: Colored reflection, high reflectivity, no refraction

### Dielectric (Glass)
```rust
fn sample_dielectric(incident: Vec3, normal: Vec3, ior: f32) -> (Vec3, f32) {
    let fresnel = schlick_fresnel(incident, normal, ior);

    if random() < fresnel {
        (reflect(incident, normal), fresnel)
    } else {
        (refract(incident, normal, 1.0 / ior).unwrap(), 1.0 - fresnel)
    }
}
```
**Properties**: Both reflection and refraction, Fresnel effect

## Normal Mapping

### Tangent Space Transformation
Normal maps are stored in tangent space, must transform to world space.

```rust
fn apply_normal_map(
    geometric_normal: Vec3,
    tangent: Vec3,
    bitangent: Vec3,
    normal_map_sample: Vec3
) -> Vec3 {
    // Normal map RGB to [-1, 1] range
    let mapped = normal_map_sample * 2.0 - 1.0;

    // Transform to world space
    let world_normal = tangent * mapped.x
                     + bitangent * mapped.y
                     + geometric_normal * mapped.z;

    world_normal.normalize()
}
```

### Geometric vs Shading Normal
- **Geometric normal**: Prevents light leaks, used for ray offset
- **Shading normal**: Visual detail, used for lighting calculations

```rust
fn shade_with_normal_map(hit: &Hit) -> Vec3 {
    let shading_normal = sample_normal_map(hit.uv);
    let geometric_normal = hit.normal;

    // Use shading normal for lighting
    let light_contrib = compute_lighting(shading_normal, ...);

    // Use geometric normal for ray offset (prevent light leaks)
    let shadow_origin = hit.point + geometric_normal * EPSILON;

    light_contrib
}
```

## Facts

### Reflection Properties
- Perfect reflection follows law: incident angle equals reflection angle
- Both incident ray and normal must be normalized for correct reflection
- Reflection depth should be limited to 3-8 bounces to prevent infinite recursion
- Each bounce reduces energy by material absorption and Fresnel effect

### Refraction Properties
- Snell's law: `n₁ × sin(θ₁) = n₂ × sin(θ₂)`
- Total internal reflection occurs when `sin²(θₜ) > 1` (dense to less dense)
- Flip normal when ray exits object (dot product < 0)
- Track ray inside/outside state to determine correct IOR ratio

### Material Characteristics
- Metals absorb and re-emit colored light (colored reflections)
- Dielectrics (glass, plastic) reflect white light regardless of base color
- Fresnel effect: ~4% reflection at normal incidence, 100% at grazing angles
- Roughness 0.0 = mirror, 0.5 = satin, 1.0 = matte

### IOR Reference Values
- Air: 1.0, Water: 1.333, Glass: 1.5-1.9, Diamond: 2.417
- Higher IOR = stronger refraction bending
- IOR ratio eta = `n₁ / n₂` (entering to exiting)

### Transparency & Absorption
- Glass needs both reflection AND refraction rays (not mutually exclusive)
- Beer's law: `I = I₀ × e^(-absorption × distance)`
- Colored glass absorbs certain wavelengths more than others
- Thin objects need special handling (single-sided transparency)

### Normal Mapping
- Normal maps stored in tangent space (RGB [0,1] → XYZ [-1,1])
- Geometric normal prevents light leaks, shading normal adds detail
- Calculate tangent space per-triangle using UV derivatives
- Always normalize final transformed normal

## Links
- [Core Concepts](./core-concepts) - Ray fundamentals and intersections
- [Lighting & Shadows](./lighting-shadows) - Light interaction with materials
- [Color & Sampling](./color-sampling) - HDR and color space considerations
- [Advanced Techniques](./advanced-techniques) - Path tracing and BRDF sampling
