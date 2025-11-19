use crate::core::bvh::BVHNode;
use crate::core::perf_test::{PerfResult, PerfSuite, PerfTest};
use crate::core::sphere::SphereData;
use crate::core::triangle_intersection::{
    moller_trumbore_intersect, watertight_intersect,
};
use crate::types::TriangleData;
use glam::Vec3;

/// Configuration for acceleration structure benchmarks
#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    pub num_primitives: usize,
    pub num_rays: usize,
    pub warmup_iterations: usize,
    pub test_iterations: usize,
    pub scene_type: SceneType,
}

#[derive(Clone, Debug)]
pub enum SceneType {
    UniformGrid,
    Clustered,
    Random,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            num_primitives: 1000,
            num_rays: 10000,
            warmup_iterations: 5,
            test_iterations: 20,
            scene_type: SceneType::Random,
        }
    }
}

/// Generate test spheres based on scene type
pub fn generate_test_spheres(count: usize, scene_type: &SceneType) -> Vec<SphereData> {
    let mut spheres = Vec::with_capacity(count);

    match scene_type {
        SceneType::UniformGrid => {
            let grid_size = (count as f32).cbrt() as usize;
            for x in 0..grid_size {
                for y in 0..grid_size {
                    for z in 0..grid_size {
                        let center = Vec3::new(
                            x as f32 * 3.0 - grid_size as f32 * 1.5,
                            y as f32 * 3.0 - grid_size as f32 * 1.5,
                            z as f32 * 3.0 - grid_size as f32 * 1.5,
                        );
                        spheres.push(SphereData::new(center, 1.0, [1.0, 0.0, 0.0]));
                        if spheres.len() >= count {
                            return spheres;
                        }
                    }
                }
            }
        }
        SceneType::Clustered => {
            let num_clusters = 10;
            let per_cluster = count / num_clusters;
            for cluster in 0..num_clusters {
                let cluster_center = Vec3::new(
                    (cluster as f32 * 20.0) - 100.0,
                    ((cluster * 3) % 7) as f32 * 10.0,
                    ((cluster * 7) % 11) as f32 * 10.0,
                );
                for i in 0..per_cluster {
                    let offset = Vec3::new(
                        (i % 10) as f32 - 5.0,
                        ((i / 10) % 10) as f32 - 5.0,
                        (i / 100) as f32 - 5.0,
                    );
                    spheres.push(SphereData::new(
                        cluster_center + offset,
                        0.5,
                        [1.0, 0.5, 0.0],
                    ));
                }
            }
        }
        SceneType::Random => {
            for i in 0..count {
                let x = ((i * 7919) % 10000) as f32 / 100.0 - 50.0;
                let y = ((i * 6547) % 10000) as f32 / 100.0 - 50.0;
                let z = ((i * 4231) % 10000) as f32 / 100.0 - 50.0;
                spheres.push(SphereData::new(Vec3::new(x, y, z), 1.0, [0.0, 1.0, 0.0]));
            }
        }
    }

    spheres
}

/// Generate test triangles
pub fn generate_test_triangles(count: usize) -> Vec<TriangleData> {
    let mut triangles = Vec::with_capacity(count);

    for i in 0..count {
        let x = ((i * 7919) % 10000) as f32 / 100.0 - 50.0;
        let y = ((i * 6547) % 10000) as f32 / 100.0 - 50.0;
        let z = ((i * 4231) % 10000) as f32 / 100.0 - 50.0;

        let v0 = [x, y, z];
        let v1 = [x + 1.0, y, z];
        let v2 = [x, y + 1.0, z];

        triangles.push(TriangleData::new(
            v0,
            v1,
            v2,
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            0,
        ));
    }

    triangles
}

/// Generate test rays
pub fn generate_test_rays(count: usize) -> Vec<(Vec3, Vec3)> {
    let mut rays = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f32 / count as f32) * 2.0 * std::f32::consts::PI;
        let height = ((i * 997) % count) as f32 / count as f32 * 100.0 - 50.0;

        let origin = Vec3::new(100.0 * angle.cos(), height, 100.0 * angle.sin());
        let dir = (Vec3::ZERO - origin).normalize();

        rays.push((origin, dir));
    }

    rays
}

/// Benchmark BVH construction
pub fn benchmark_bvh_construction(config: &BenchmarkConfig) -> PerfResult {
    let spheres = generate_test_spheres(config.num_primitives, &config.scene_type);

    PerfTest::new("BVH Construction")
        .with_warmup(config.warmup_iterations)
        .with_iterations(config.test_iterations)
        .run(|| {
            let bvh = BVHNode::build(&spheres);
            std::hint::black_box(bvh);
        })
}

/// Benchmark BVH traversal
pub fn benchmark_bvh_traversal(config: &BenchmarkConfig) -> PerfResult {
    let spheres = generate_test_spheres(config.num_primitives, &config.scene_type);
    let bvh = BVHNode::build(&spheres);
    let rays = generate_test_rays(config.num_rays);

    PerfTest::new("BVH Traversal")
        .with_warmup(config.warmup_iterations)
        .with_iterations(config.test_iterations)
        .run(|| {
            for (origin, dir) in &rays {
                // Simplified traversal test
                let _ = traverse_bvh(&bvh, &spheres, *origin, *dir);
            }
        })
}

/// Simple BVH traversal (for benchmarking)
fn traverse_bvh(
    node: &BVHNode,
    spheres: &[SphereData],
    ray_origin: Vec3,
    ray_dir: Vec3,
) -> Option<f32> {
    if !intersect_aabb(node.bounds(), ray_origin, ray_dir) {
        return None;
    }

    match node {
        BVHNode::Leaf {
            primitive_indices, ..
        } => {
            let mut closest = None;
            let mut closest_t = f32::INFINITY;

            for &idx in primitive_indices {
                if let Some(t) = spheres[idx as usize].intersect(ray_origin, ray_dir) {
                    if t < closest_t {
                        closest_t = t;
                        closest = Some(t);
                    }
                }
            }

            closest
        }
        BVHNode::Internal { left, right, .. } => {
            let hit_left = traverse_bvh(left, spheres, ray_origin, ray_dir);
            let hit_right = traverse_bvh(right, spheres, ray_origin, ray_dir);

            match (hit_left, hit_right) {
                (Some(t1), Some(t2)) => Some(t1.min(t2)),
                (Some(t), None) | (None, Some(t)) => Some(t),
                (None, None) => None,
            }
        }
    }
}

/// Simple AABB intersection test
fn intersect_aabb(bounds: &crate::math::AABB, ray_origin: Vec3, ray_dir: Vec3) -> bool {
    let inv_dir = 1.0 / ray_dir;
    let t1 = (bounds.min - ray_origin) * inv_dir;
    let t2 = (bounds.max - ray_origin) * inv_dir;

    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();

    tmax >= tmin && tmax >= 0.0
}

/// Benchmark triangle intersection algorithms
pub fn benchmark_triangle_intersection(config: &BenchmarkConfig) -> PerfSuite {
    let mut suite = PerfSuite::new("Triangle Intersection Algorithms");

    let triangles = generate_test_triangles(config.num_primitives);
    let rays = generate_test_rays(config.num_rays);

    // Benchmark Möller-Trumbore
    let moller_result = PerfTest::new("Möller-Trumbore")
        .with_warmup(config.warmup_iterations)
        .with_iterations(config.test_iterations)
        .run(|| {
            for (origin, dir) in &rays {
                for tri in &triangles {
                    let _ = moller_trumbore_intersect(
                        *origin,
                        *dir,
                        Vec3::from_array(tri.v0),
                        Vec3::from_array(tri.v1),
                        Vec3::from_array(tri.v2),
                    );
                }
            }
        });

    // Benchmark Watertight
    let watertight_result = PerfTest::new("Watertight")
        .with_warmup(config.warmup_iterations)
        .with_iterations(config.test_iterations)
        .run(|| {
            for (origin, dir) in &rays {
                for tri in &triangles {
                    let _ = watertight_intersect(
                        *origin,
                        *dir,
                        Vec3::from_array(tri.v0),
                        Vec3::from_array(tri.v1),
                        Vec3::from_array(tri.v2),
                    );
                }
            }
        });

    suite.add_result(moller_result);
    suite.add_result(watertight_result);

    suite
}

/// Run full benchmark suite
pub fn run_full_benchmark_suite() {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║     Ray Tracing Acceleration Structure Benchmarks     ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    let configs = vec![
        ("Small Scene (100 prims)", BenchmarkConfig {
            num_primitives: 100,
            num_rays: 1000,
            ..Default::default()
        }),
        ("Medium Scene (1K prims)", BenchmarkConfig {
            num_primitives: 1000,
            num_rays: 10000,
            ..Default::default()
        }),
        ("Large Scene (10K prims)", BenchmarkConfig {
            num_primitives: 10000,
            num_rays: 10000,
            ..Default::default()
        }),
    ];

    for (name, config) in configs {
        println!("\n{}", name);
        println!("{}", "=".repeat(50));

        println!("\n[BVH Construction]");
        let construction = benchmark_bvh_construction(&config);
        construction.print_summary();

        println!("\n[BVH Traversal]");
        let traversal = benchmark_bvh_traversal(&config);
        traversal.print_summary();

        let ops_per_sec = config.num_rays as f64 / traversal.avg_duration.as_secs_f64();
        println!("Throughput: {:.2} Mrays/sec", ops_per_sec / 1_000_000.0);
    }

    // Triangle intersection comparison
    println!("\n\n[Triangle Intersection Algorithms]");
    let config = BenchmarkConfig {
        num_primitives: 100,
        num_rays: 100,
        test_iterations: 50,
        ..Default::default()
    };

    let tri_suite = benchmark_triangle_intersection(&config);
    tri_suite.print_comparison();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_spheres_uniform() {
        let spheres = generate_test_spheres(27, &SceneType::UniformGrid);
        assert_eq!(spheres.len(), 27);
    }

    #[test]
    fn test_generate_spheres_clustered() {
        let spheres = generate_test_spheres(100, &SceneType::Clustered);
        assert_eq!(spheres.len(), 100);
    }

    #[test]
    fn test_generate_spheres_random() {
        let spheres = generate_test_spheres(50, &SceneType::Random);
        assert_eq!(spheres.len(), 50);
    }

    #[test]
    fn test_generate_triangles() {
        let triangles = generate_test_triangles(100);
        assert_eq!(triangles.len(), 100);
    }

    #[test]
    fn test_generate_rays() {
        let rays = generate_test_rays(50);
        assert_eq!(rays.len(), 50);

        for (_, dir) in rays {
            assert!((dir.length() - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_bvh_construction_benchmark() {
        let config = BenchmarkConfig {
            num_primitives: 10,
            num_rays: 10,
            warmup_iterations: 2,
            test_iterations: 3,
            scene_type: SceneType::Random,
        };

        let result = benchmark_bvh_construction(&config);
        assert_eq!(result.iterations, 3);
        assert!(result.avg_duration.as_nanos() > 0);
    }

    #[test]
    fn test_bvh_traversal_benchmark() {
        let config = BenchmarkConfig {
            num_primitives: 10,
            num_rays: 10,
            warmup_iterations: 2,
            test_iterations: 3,
            scene_type: SceneType::Random,
        };

        let result = benchmark_bvh_traversal(&config);
        assert_eq!(result.iterations, 3);
        assert!(result.avg_duration.as_nanos() > 0);
    }

    #[test]
    fn test_aabb_intersection() {
        use crate::math::AABB;

        let bounds = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));

        // Hit
        let hit = intersect_aabb(&bounds, Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(hit);

        // Miss
        let miss = intersect_aabb(&bounds, Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(!miss);
    }

    #[test]
    fn test_bvh_traversal() {
        let spheres = vec![
            SphereData::new(Vec3::new(0.0, 0.0, -5.0), 1.0, [1.0, 0.0, 0.0]),
            SphereData::new(Vec3::new(5.0, 0.0, -5.0), 1.0, [0.0, 1.0, 0.0]),
        ];

        let bvh = BVHNode::build(&spheres);

        let hit = traverse_bvh(&bvh, &spheres, Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert!(hit.is_some());

        let miss = traverse_bvh(&bvh, &spheres, Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        assert!(miss.is_none());
    }
}
