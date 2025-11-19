# AABB (Axis-Aligned Bounding Boxes)

## Overview

Axis-Aligned Bounding Boxes are the workhorse of spatial acceleration in ray tracing. Unlike [Bounding Spheres](BOUNDING_SPHERES.md), AABBs align with coordinate axes, enabling extremely fast intersection tests and efficient [BVH construction](BVH.md).

## Mathematical Definition

An AABB is defined by two points:
- **Min**: `(min_x, min_y, min_z)` - lower corner
- **Max**: `(max_x, max_y, max_z)` - upper corner

Any point `P = (x, y, z)` is inside if:
```
min_x ≤ x ≤ max_x  AND
min_y ≤ y ≤ max_y  AND
min_z ≤ z ≤ max_z
```

## Compact Representation

```rust
#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn centroid(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn extent(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn surface_area(&self) -> f32 {
        let e = self.extent();
        2.0 * (e.x * e.y + e.y * e.z + e.z * e.x)
    }
}
```

## Ray-AABB Intersection

The "slab method" is the canonical approach, testing ray intersection against each pair of parallel planes.

### Amy Williams' Optimization

The most cache-efficient implementation uses precomputed inverse direction:

```rust
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_direction: Vec3,  // Precomputed 1.0 / direction
    pub sign: [usize; 3],     // Precomputed for branchless traversal
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let inv_direction = Vec3::new(
            1.0 / direction.x,
            1.0 / direction.y,
            1.0 / direction.z,
        );
        let sign = [
            (inv_direction.x < 0.0) as usize,
            (inv_direction.y < 0.0) as usize,
            (inv_direction.z < 0.0) as usize,
        ];
        Self { origin, direction, inv_direction, sign }
    }
}

pub fn intersect_aabb(ray: &Ray, aabb: &AABB, t_min: f32, t_max: f32) -> bool {
    let bounds = [aabb.min, aabb.max];

    let mut tmin = (bounds[ray.sign[0]].x - ray.origin.x) * ray.inv_direction.x;
    let mut tmax = (bounds[1 - ray.sign[0]].x - ray.origin.x) * ray.inv_direction.x;

    let tymin = (bounds[ray.sign[1]].y - ray.origin.y) * ray.inv_direction.y;
    let tymax = (bounds[1 - ray.sign[1]].y - ray.origin.y) * ray.inv_direction.y;

    if tmin > tymax || tymin > tmax {
        return false;
    }

    tmin = tmin.max(tymin);
    tmax = tmax.min(tymax);

    let tzmin = (bounds[ray.sign[2]].z - ray.origin.z) * ray.inv_direction.z;
    let tzmax = (bounds[1 - ray.sign[2]].z - ray.origin.z) * ray.inv_direction.z;

    if tmin > tzmax || tzmin > tmax {
        return false;
    }

    tmin = tmin.max(tzmin);
    tmax = tmax.min(tzmax);

    tmin < t_max && tmax > t_min
}
```

### SIMD Optimization

Modern CPUs can test 4 AABBs simultaneously:

```rust
use std::simd::*;

pub fn intersect_aabb_4(
    ray: &Ray,
    aabb_min: [f32x4; 3],
    aabb_max: [f32x4; 3],
) -> u32 {
    let origin = [
        f32x4::splat(ray.origin.x),
        f32x4::splat(ray.origin.y),
        f32x4::splat(ray.origin.z),
    ];
    let inv_dir = [
        f32x4::splat(ray.inv_direction.x),
        f32x4::splat(ray.inv_direction.y),
        f32x4::splat(ray.inv_direction.z),
    ];

    let mut tmin = (aabb_min[0] - origin[0]) * inv_dir[0];
    let mut tmax = (aabb_max[0] - origin[0]) * inv_dir[0];

    // Continue for y and z...
    // Returns bitmask of which AABBs intersect
    tmin.simd_le(tmax).to_bitmask()
}
```

## AABB Construction

### From Primitives

```rust
impl AABB {
    pub fn from_triangle(v0: Vec3, v1: Vec3, v2: Vec3) -> Self {
        Self {
            min: v0.min(v1).min(v2),
            max: v0.max(v1).max(v2),
        }
    }

    pub fn from_sphere(center: Vec3, radius: f32) -> Self {
        let r = Vec3::splat(radius);
        Self {
            min: center - r,
            max: center + r,
        }
    }
}
```

### Union Operations

Essential for [BVH](BVH.md) construction:

```rust
impl AABB {
    pub fn union(&self, other: &AABB) -> AABB {
        AABB {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn grow(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    pub fn expand(&self, epsilon: f32) -> AABB {
        let e = Vec3::splat(epsilon);
        AABB {
            min: self.min - e,
            max: self.max + e,
        }
    }
}
```

## Surface Area Heuristic (SAH)

Critical for optimal [BVH](BVH.md) construction. The cost function balances traversal vs intersection:

```
Cost = C_traversal + (SA_left / SA_parent) * N_left * C_intersect
                   + (SA_right / SA_parent) * N_right * C_intersect
```

```rust
pub fn evaluate_sah(
    left_bounds: &AABB,
    left_count: usize,
    right_bounds: &AABB,
    right_count: usize,
) -> f32 {
    const TRAVERSAL_COST: f32 = 1.0;
    const INTERSECTION_COST: f32 = 1.0;

    let left_area = left_bounds.surface_area();
    let right_area = right_bounds.surface_area();
    let total_area = left_bounds.union(right_bounds).surface_area();

    TRAVERSAL_COST
        + (left_area / total_area) * left_count as f32 * INTERSECTION_COST
        + (right_area / total_area) * right_count as f32 * INTERSECTION_COST
}
```

## Performance Characteristics

### Pros
- **Blazing fast intersection**: ~20 instructions, highly predictable
- **Tight fitting**: Minimal wasted space for axis-aligned geometry
- **Easy union**: Componentwise min/max operations
- **SIMD friendly**: 4-wide or 8-wide tests trivial
- **Cache coherent**: 6 floats, perfect for cache lines

### Cons
- **Rotation sensitive**: Bounding volume grows with rotation
- **Poor for diagonal geometry**: Wastes space on 45° oriented objects
- **Not rotation invariant**: Requires recomputation on transform

## Oriented Bounding Boxes (OBB)

For rotated geometry, use OBBs (see ):
```rust
pub struct OBB {
    pub center: Vec3,
    pub axes: [Vec3; 3],    // Local coordinate frame
    pub half_extents: Vec3,
}
```

## Memory Layout for BVH

Cache-friendly SoA layout for [BVH](BVH.md) nodes:

```rust
#[repr(C, align(64))]  // Cache line aligned
pub struct BVHNode {
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub left_child: u32,   // Index to left child or first primitive

    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
    pub right_child: u32,  // Index to right child or primitive count
}
```

This layout enables:
- Single cache line fetch per node
- SIMD-friendly data access
- Minimal memory bandwidth

## Use Cases

1. **[BVH](BVH.md) nodes**: Industry standard for acceleration structure
2. **Frustum culling**: Fast camera visibility tests
3. **Broad phase collision**: Physics engine spatial partitioning
4. **Level-of-detail**: Distance-based detail switching
5. **Occlusion culling**: Conservative visibility determination

## Related Topics

- [BVH](BVH.md) - Hierarchical spatial acceleration using AABBs
- [Bounding Spheres](BOUNDING_SPHERES.md) - Rotation-invariant alternative
-  - Surface Area Heuristic for optimal BVH construction
-  - Oriented bounding boxes for rotated geometry
-  - Grid, octree, and other structures

## References

- Williams et al. "An Efficient and Robust Ray-Box Intersection Algorithm" (2005)
- Wald, Ingo. "On fast Construction of SAH-based Bounding Volume Hierarchies" (2007)
- Akenine-Möller et al. "Real-Time Rendering" 4th Ed. (2018)
