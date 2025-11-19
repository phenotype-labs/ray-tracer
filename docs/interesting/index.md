# Interesting Topics

Deep dives into advanced ray tracing concepts, optimizations, and implementation details.

## Acceleration Structures

Ray tracing performance lives and dies by efficient spatial acceleration. These topics explore the fundamental structures that make real-time ray tracing possible.

### [AABB (Axis-Aligned Bounding Boxes)](./aabb)

The foundation of spatial acceleration. Learn about:
- Fast ray-AABB intersection tests
- Surface Area Heuristic (SAH)
- SIMD optimizations
- Memory layouts for cache efficiency

### [BVH (Bounding Volume Hierarchies)](./bvh)

The dominant acceleration structure in production ray tracers. Covers:
- Construction algorithms (SAH, SBVH)
- Traversal strategies (stack-based, stackless)
- Two-level hierarchies (TLAS/BLAS)
- Compressed and quantized BVHs

### [Bounding Spheres](./bounding-spheres)

Rotation-invariant bounding primitives. Explores:
- Ray-sphere intersection
- Ritter's and Welzl's algorithms
- Performance trade-offs vs AABB
- Hybrid approaches

---

## Performance Philosophy

> "After profiling the physics engine, I realized the bottleneck wasn't the quaternion slerp itself but cache misses from my array-of-structs layout, so I switched to structure-of-arrays with SIMD intrinsics and now I'm processing 100k skeletal transforms at 2ms per frame."

This documentation emphasizes **practical performance** over theoretical elegance. Every technique discussed has been measured, profiled, and battle-tested.
