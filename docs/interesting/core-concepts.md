# Core Ray Tracing Concepts

ray-tracing-fundamentals, ray-intersection, camera-rays, scene-geometry

## Ray Fundamentals

### Ray Structure
A ray consists of two components:
- **Origin point** (P₀): Starting position in 3D space
- **Direction vector** (d): Normalized unit vector indicating ray path

The parametric ray equation: `P(t) = P₀ + t·d` where t ≥ 0

### Critical Implementation Details
- **Direction normalization is mandatory** - unnormalized directions break distance calculations
- **Camera rays originate from camera position, not the screen plane** - the screen is a target, not a source
- **Every pixel shoots at least one ray** - this is the fundamental rendering primitive
- **Rays can miss** - always handle null intersections gracefully

## Intersection Testing

### Primitive Ordering by Difficulty
1. **Ray-Sphere** (easiest): Quadratic equation, closed-form solution
2. **Ray-Plane**: Fundamental building block, single dot product test
3. **Ray-Triangle**: Most common, barycentric coordinates needed
4. **Ray-AABB**: Critical for acceleration structures

### Implement Ray-Plane First
Plane intersection forms the foundation for understanding:
- Normal vectors
- Distance calculations
- Hit point derivation

```rust
fn ray_plane_intersection(ray_origin: Vec3, ray_dir: Vec3, plane_normal: Vec3, plane_d: f32) -> Option<f32> {
    let denom = ray_dir.dot(plane_normal);
    if denom.abs() < EPSILON { return None; } // Parallel

    let t = -(ray_origin.dot(plane_normal) + plane_d) / denom;
    if t >= 0.0 { Some(t) } else { None }
}
```

## Distance and Hit Management

### Closest Hit Algorithm
**Never store just any hit - track the closest**

```rust
let mut closest_t = f32::INFINITY;
let mut closest_hit = None;

for object in scene {
    if let Some(t) = ray_intersect(ray, object) {
        if t < closest_t {
            closest_t = t;
            closest_hit = Some(object);
        }
    }
}
```

### Ray Distance Bounds
- **t_min**: Typically EPSILON to avoid self-intersection
- **t_max**: Maximum ray distance, prevents far plane artifacts
- Early exit when `t > closest_t` (no point continuing)

## Camera Ray Generation

### Field of View Calculation
FOV affects ray direction, not origin:

```rust
fn generate_camera_ray(pixel_x: u32, pixel_y: u32, fov: f32, width: u32, height: u32) -> Ray {
    let aspect = width as f32 / height as f32;
    let fov_rad = fov.to_radians();
    let scale = (fov_rad / 2.0).tan();

    // NDC: [-1, 1] range
    let ndc_x = (2.0 * pixel_x as f32 / width as f32 - 1.0) * aspect * scale;
    let ndc_y = (1.0 - 2.0 * pixel_y as f32 / height as f32) * scale;

    let direction = Vec3::new(ndc_x, ndc_y, -1.0).normalize();

    Ray {
        origin: camera_position,
        direction
    }
}
```

### Pixel Sampling Strategy
- **Center sampling**: Fast but aliased
- **Jittered sampling**: Randomize within pixel bounds for antialiasing
- **Stratified sampling**: Subdivide pixel into grid for uniform coverage

## Bounding Volume Acceleration

### Why Bounding Volumes Matter
Testing every object for every ray = O(n·m) complexity disaster

**Solution**: Hierarchical bounding volumes (BVH)
- Test ray against bounding volume first
- Only test contained objects if bounding volume hits
- Reduces to O(log n) average case

### Bounding Volume Priority
1. **AABB (Axis-Aligned Bounding Boxes)**: Industry standard, cache-friendly
2. **Bounding Spheres**: Rotation-invariant but less tight fit
3. **OBB (Oriented Bounding Boxes)**: Tighter fit but more expensive

See [AABB](./aabb) and [BVH](./bvh) for deep dives.

## Facts

### Intersection Testing
- Ray-sphere intersection uses quadratic formula: `b²-4ac` discriminant
- Ray-plane intersection requires single dot product
- AABB tests use slab method (6 plane intersections optimized to 3)
- Möller-Trumbore algorithm solves ray-triangle in one pass

### Performance Characteristics
- BVH reduces intersection tests from O(n) to O(log n)
- Build BVH only when scene changes, not per-frame
- Precompute inverse transformation matrices for static objects
- Cache ray direction calculations when possible

### Ray Properties
- Ray direction must be normalized for accurate distance calculations
- Maximum ray distance prevents precision issues at infinity
- Epsilon offset (0.001) prevents self-intersection artifacts
- Camera rays use perspective projection, not orthographic

## Links
- [AABB (Axis-Aligned Bounding Boxes)](./aabb)
- [BVH (Bounding Volume Hierarchies)](./bvh)
- [Bounding Spheres](./bounding-spheres)
- [Lighting & Shadows](./lighting-shadows)
