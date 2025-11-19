# Bounding Spheres

## Overview

Bounding spheres are fundamental geometric primitives used in ray tracing and collision detection. Unlike axis-aligned bounding boxes (AABBs), spheres provide a rotation-invariant bound that can be more efficient for certain types of geometry and transformations.

## Mathematical Definition

A bounding sphere is defined by:
- **Center**: Position vector `C = (cx, cy, cz)`
- **Radius**: Scalar value `r`

Any point `P` is inside the sphere if: `|P - C| ≤ r`

## Ray-Sphere Intersection

The ray-sphere intersection test is one of the most elegant in computational geometry.

Given:
- Ray origin: `O`
- Ray direction: `D` (normalized)
- Sphere center: `C`
- Sphere radius: `r`

The parametric ray equation is: `P(t) = O + tD`

Substituting into the sphere equation `|P - C|² = r²`:

```
|O + tD - C|² = r²
```

Expanding:
```
(D·D)t² + 2D·(O-C)t + (O-C)·(O-C) - r² = 0
```

This is a quadratic equation: `at² + bt + c = 0` where:
- `a = D·D = 1` (since D is normalized)
- `b = 2D·(O-C)`
- `c = (O-C)·(O-C) - r²`

### Discriminant Analysis

```
discriminant = b² - 4ac
```

- `discriminant < 0`: No intersection
- `discriminant = 0`: Tangent (one intersection)
- `discriminant > 0`: Two intersections

### Optimized Implementation

```rust
pub fn intersect_sphere(
    ray_origin: Vec3,
    ray_dir: Vec3,
    sphere_center: Vec3,
    sphere_radius: f32,
) -> Option<f32> {
    let oc = ray_origin - sphere_center;
    let b = oc.dot(ray_dir);
    let c = oc.length_squared() - sphere_radius * sphere_radius;

    let discriminant = b * b - c;

    if discriminant < 0.0 {
        return None;
    }

    // Return nearest intersection
    let t = -b - discriminant.sqrt();
    if t > 0.0 {
        Some(t)
    } else {
        None
    }
}
```

## Bounding Sphere Construction

### Ritter's Algorithm

A fast approximate algorithm for computing minimal bounding spheres:

1. Find point `P` with maximum distance from arbitrary point
2. Find point `Q` with maximum distance from `P`
3. Initial sphere: center at midpoint of `P-Q`, radius = `|P-Q|/2`
4. Grow sphere to encompass all points outside it

```rust
pub fn ritter_bounding_sphere(points: &[Vec3]) -> (Vec3, f32) {
    if points.is_empty() {
        return (Vec3::ZERO, 0.0);
    }

    // Find two distant points
    let p = points[0];
    let q = points.iter()
        .max_by(|a, b| {
            a.distance_squared(p)
                .partial_cmp(&b.distance_squared(p))
                .unwrap()
        })
        .unwrap();

    let mut center = (p + *q) * 0.5;
    let mut radius = p.distance(*q) * 0.5;

    // Expand to include all points
    for &point in points {
        let dist = point.distance(center);
        if dist > radius {
            let new_radius = (radius + dist) * 0.5;
            let offset = (dist - radius) / dist;
            center = center + (point - center) * offset;
            radius = new_radius;
        }
    }

    (center, radius)
}
```

### Welzl's Algorithm

Exact minimal bounding sphere in expected O(n) time using randomization:

```rust
pub fn welzl_sphere(points: &[Vec3]) -> (Vec3, f32) {
    fn sphere_from_boundary(boundary: &[Vec3]) -> (Vec3, f32) {
        match boundary.len() {
            0 => (Vec3::ZERO, 0.0),
            1 => (boundary[0], 0.0),
            2 => {
                let center = (boundary[0] + boundary[1]) * 0.5;
                let radius = boundary[0].distance(center);
                (center, radius)
            }
            3 => circumsphere_3(boundary[0], boundary[1], boundary[2]),
            4 => circumsphere_4(boundary[0], boundary[1], boundary[2], boundary[3]),
            _ => unreachable!(),
        }
    }

    // Recursive implementation (simplified)
    welzl_recursive(points, &[], points.len())
}
```

## Performance Characteristics

### Pros
- **Rotation invariant**: No need to recompute on rotation
- **Simple intersection**: Elegant quadratic solution
- **Cache friendly**: Only 4 floats (center + radius)
- **SIMD friendly**: Parallel sphere tests trivial

### Cons
- **Loose fitting**: Often wastes more space than AABBs
- **Not hierarchical**: Harder to build efficient BVH structures
- **Poor for thin geometry**: Terrible fit for planes, long triangles

## Use Cases in Ray Tracing

1. **Coarse culling**: Fast first-pass rejection
2. **Level-of-detail**: Sphere size determines detail level
3. **Particle systems**: Natural fit for spherical particles
4. **Character bounds**: Good for humanoid shapes
5. **Probe placement**: Environment map/light probe positioning

## Hybrid Approaches

Combine with other primitives:
- **Sphere-AABB hierarchy**: Spheres at leaves, AABBs for internal nodes
- **Capsules**: Extended spheres for elongated objects
- **Ellipsoids**: Scaled spheres for directional bounds

## References

- Ericson, Christer. "Real-Time Collision Detection" (2004)
- Welzl, Emo. "Smallest enclosing disks (balls and ellipsoids)" (1991)
- Ritter, Jack. "An Efficient Bounding Sphere" (1990)
