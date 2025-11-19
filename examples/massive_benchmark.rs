use ray_tracer::core::benchmark::*;
use ray_tracer::core::bvh::{BVHNode, BVHPrimitive};
use ray_tracer::core::perf_test::PerfTest;
use ray_tracer::core::sphere::SphereData;
use ray_tracer::core::triangle_intersection::moller_trumbore_intersect;
use ray_tracer::types::TriangleData;
use glam::Vec3;

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║       MASSIVE RAY TRACING BENCHMARK SUITE                 ║");
    println!("║    Testing with 100K, 1M, and 10M triangles/spheres      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Benchmark configurations
    let configs = vec![
        ("100K Triangles", 100_000, 50_000),
        ("1M Triangles", 1_000_000, 100_000),
        ("10M Triangles", 10_000_000, 100_000),
    ];

    for (name, num_triangles, num_rays) in configs {
        println!("\n{:=<60}", "");
        println!("{}  [{}]", name, chrono::Local::now().format("%H:%M:%S"));
        println!("{:=<60}", "");

        run_massive_benchmark(num_triangles, num_rays);
    }

    // Sphere benchmarks
    println!("\n\n╔════════════════════════════════════════════════════════════╗");
    println!("║              SPHERE BENCHMARKS                            ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let sphere_configs = vec![
        ("100K Spheres", 100_000, 50_000),
        ("1M Spheres", 1_000_000, 100_000),
    ];

    for (name, num_spheres, num_rays) in sphere_configs {
        println!("\n{:=<60}", "");
        println!("{}  [{}]", name, chrono::Local::now().format("%H:%M:%S"));
        println!("{:=<60}", "");

        run_sphere_benchmark(num_spheres, num_rays);
    }

    // BVH vs Linear comparison
    println!("\n\n╔════════════════════════════════════════════════════════════╗");
    println!("║         BVH vs LINEAR TRAVERSAL COMPARISON               ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    run_bvh_vs_linear_comparison();
}

fn run_massive_benchmark(num_triangles: usize, num_rays: usize) {
    println!("[1/4] Generating {} triangles...", num_triangles);
    let start = std::time::Instant::now();
    let triangles = generate_test_triangles(num_triangles);
    println!("      Generated in {:.2}s", start.elapsed().as_secs_f64());

    println!("[2/4] Building BVH...");
    let start = std::time::Instant::now();

    // Create wrapper for BVH
    let triangle_wrappers: Vec<TriangleWrapper> = triangles
        .iter()
        .map(|t| TriangleWrapper(*t))
        .collect();

    let bvh = BVHNode::build(&triangle_wrappers);
    let build_time = start.elapsed();
    println!("      Built in {:.2}s", build_time.as_secs_f64());

    let stats = bvh.stats();
    println!("\n      BVH Statistics:");
    println!("      - Nodes: {}", stats.num_nodes);
    println!("      - Leaves: {}", stats.num_leaves);
    println!("      - Max Depth: {}", stats.max_depth);
    println!("      - Avg Leaf Size: {:.2}", stats.avg_leaf_size);

    println!("\n[3/4] Generating {} rays...", num_rays);
    let rays = generate_test_rays(num_rays);

    println!("[4/4] Running traversal benchmark...");

    let result = PerfTest::new("BVH Traversal")
        .with_warmup(2)
        .with_iterations(5)
        .run(|| {
            let mut hit_count = 0;
            for (origin, dir) in &rays {
                if traverse_bvh_triangles(&bvh, &triangles, *origin, *dir).is_some() {
                    hit_count += 1;
                }
            }
            std::hint::black_box(hit_count);
        });

    println!("\n      ╔════════════════════════════════════════════════╗");
    println!("      ║  RESULTS                                       ║");
    println!("      ╠════════════════════════════════════════════════╣");
    println!("      ║  Avg Time:     {:>10.2} ms                ║", result.avg_duration.as_secs_f64() * 1000.0);
    println!("      ║  Min Time:     {:>10.2} ms                ║", result.min_duration.as_secs_f64() * 1000.0);
    println!("      ║  Max Time:     {:>10.2} ms                ║", result.max_duration.as_secs_f64() * 1000.0);

    let mrays_per_sec = num_rays as f64 / result.avg_duration.as_secs_f64() / 1_000_000.0;
    println!("      ║  Throughput:   {:>10.2} Mrays/sec         ║", mrays_per_sec);

    let tests_per_ray = stats.avg_leaf_size * stats.max_depth as f32;
    println!("      ║  Tests/Ray:    {:>10.2}                    ║", tests_per_ray);
    println!("      ╚════════════════════════════════════════════════╝");

    // Memory usage estimate
    let bvh_memory_mb = (stats.num_nodes * 64) as f64 / 1024.0 / 1024.0;
    let tri_memory_mb = (num_triangles * std::mem::size_of::<TriangleData>()) as f64 / 1024.0 / 1024.0;
    println!("\n      Memory Usage:");
    println!("      - BVH: {:.2} MB", bvh_memory_mb);
    println!("      - Triangles: {:.2} MB", tri_memory_mb);
    println!("      - Total: {:.2} MB", bvh_memory_mb + tri_memory_mb);
}

fn run_sphere_benchmark(num_spheres: usize, num_rays: usize) {
    println!("[1/4] Generating {} spheres...", num_spheres);
    let start = std::time::Instant::now();
    let spheres = generate_test_spheres(num_spheres, &SceneType::Random);
    println!("      Generated in {:.2}s", start.elapsed().as_secs_f64());

    println!("[2/4] Building BVH...");
    let start = std::time::Instant::now();
    let bvh = BVHNode::build(&spheres);
    let build_time = start.elapsed();
    println!("      Built in {:.2}s", build_time.as_secs_f64());

    let stats = bvh.stats();
    println!("\n      BVH Statistics:");
    println!("      - Nodes: {}", stats.num_nodes);
    println!("      - Leaves: {}", stats.num_leaves);
    println!("      - Max Depth: {}", stats.max_depth);
    println!("      - Avg Leaf Size: {:.2}", stats.avg_leaf_size);

    println!("\n[3/4] Generating {} rays...", num_rays);
    let rays = generate_test_rays(num_rays);

    println!("[4/4] Running traversal benchmark...");

    let result = PerfTest::new("BVH Traversal")
        .with_warmup(2)
        .with_iterations(5)
        .run(|| {
            let mut hit_count = 0;
            for (origin, dir) in &rays {
                if traverse_bvh_spheres(&bvh, &spheres, *origin, *dir).is_some() {
                    hit_count += 1;
                }
            }
            std::hint::black_box(hit_count);
        });

    println!("\n      ╔════════════════════════════════════════════════╗");
    println!("      ║  RESULTS                                       ║");
    println!("      ╠════════════════════════════════════════════════╣");
    println!("      ║  Avg Time:     {:>10.2} ms                ║", result.avg_duration.as_secs_f64() * 1000.0);
    println!("      ║  Throughput:   {:>10.2} Mrays/sec         ║", num_rays as f64 / result.avg_duration.as_secs_f64() / 1_000_000.0);
    println!("      ╚════════════════════════════════════════════════╝");
}

fn run_bvh_vs_linear_comparison() {
    let counts = vec![1_000, 10_000, 100_000];
    let num_rays = 10_000;

    for &count in &counts {
        println!("\n{} Triangles:", count);

        let triangles = generate_test_triangles(count);
        let triangle_wrappers: Vec<TriangleWrapper> = triangles
            .iter()
            .map(|t| TriangleWrapper(*t))
            .collect();
        let bvh = BVHNode::build(&triangle_wrappers);
        let rays = generate_test_rays(num_rays);

        // BVH traversal
        let bvh_result = PerfTest::new("BVH")
            .with_warmup(2)
            .with_iterations(10)
            .run(|| {
                for (origin, dir) in &rays {
                    let _ = traverse_bvh_triangles(&bvh, &triangles, *origin, *dir);
                }
            });

        // Linear scan
        let linear_result = PerfTest::new("Linear")
            .with_warmup(2)
            .with_iterations(10)
            .run(|| {
                for (origin, dir) in &rays {
                    let _ = linear_scan(&triangles, *origin, *dir);
                }
            });

        let speedup = linear_result.avg_duration.as_secs_f64() / bvh_result.avg_duration.as_secs_f64();

        println!("  BVH:    {:>8.2} ms ({:>6.2} Mrays/sec)",
                 bvh_result.avg_duration.as_secs_f64() * 1000.0,
                 num_rays as f64 / bvh_result.avg_duration.as_secs_f64() / 1_000_000.0);
        println!("  Linear: {:>8.2} ms ({:>6.2} Mrays/sec)",
                 linear_result.avg_duration.as_secs_f64() * 1000.0,
                 num_rays as f64 / linear_result.avg_duration.as_secs_f64() / 1_000_000.0);
        println!("  Speedup: {:.2}x faster with BVH", speedup);
    }
}

// Helper functions

#[derive(Clone, Copy)]
struct TriangleWrapper(TriangleData);

impl BVHPrimitive for TriangleWrapper {
    fn bounds(&self) -> ray_tracer::math::AABB {
        self.0.bounds()
    }
}

fn traverse_bvh_triangles(
    node: &BVHNode,
    triangles: &[TriangleData],
    ray_origin: Vec3,
    ray_dir: Vec3,
) -> Option<f32> {
    if !intersect_aabb(node.bounds(), ray_origin, ray_dir) {
        return None;
    }

    match node {
        BVHNode::Leaf { primitive_indices, .. } => {
            let mut closest = None;
            let mut closest_t = f32::INFINITY;

            for &idx in primitive_indices {
                let tri = &triangles[idx as usize];
                if let Some(hit) = moller_trumbore_intersect(
                    ray_origin,
                    ray_dir,
                    Vec3::from_array(tri.v0),
                    Vec3::from_array(tri.v1),
                    Vec3::from_array(tri.v2),
                ) {
                    if hit.t < closest_t {
                        closest_t = hit.t;
                        closest = Some(hit.t);
                    }
                }
            }

            closest
        }
        BVHNode::Internal { left, right, .. } => {
            let hit_left = traverse_bvh_triangles(left, triangles, ray_origin, ray_dir);
            let hit_right = traverse_bvh_triangles(right, triangles, ray_origin, ray_dir);

            match (hit_left, hit_right) {
                (Some(t1), Some(t2)) => Some(t1.min(t2)),
                (Some(t), None) | (None, Some(t)) => Some(t),
                (None, None) => None,
            }
        }
    }
}

fn traverse_bvh_spheres(
    node: &BVHNode,
    spheres: &[SphereData],
    ray_origin: Vec3,
    ray_dir: Vec3,
) -> Option<f32> {
    if !intersect_aabb(node.bounds(), ray_origin, ray_dir) {
        return None;
    }

    match node {
        BVHNode::Leaf { primitive_indices, .. } => {
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
            let hit_left = traverse_bvh_spheres(left, spheres, ray_origin, ray_dir);
            let hit_right = traverse_bvh_spheres(right, spheres, ray_origin, ray_dir);

            match (hit_left, hit_right) {
                (Some(t1), Some(t2)) => Some(t1.min(t2)),
                (Some(t), None) | (None, Some(t)) => Some(t),
                (None, None) => None,
            }
        }
    }
}

fn intersect_aabb(bounds: &ray_tracer::math::AABB, ray_origin: Vec3, ray_dir: Vec3) -> bool {
    let inv_dir = 1.0 / ray_dir;
    let t1 = (bounds.min - ray_origin) * inv_dir;
    let t2 = (bounds.max - ray_origin) * inv_dir;

    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();

    tmax >= tmin && tmax >= 0.0
}

fn linear_scan(triangles: &[TriangleData], ray_origin: Vec3, ray_dir: Vec3) -> Option<f32> {
    let mut closest = None;
    let mut closest_t = f32::INFINITY;

    for tri in triangles {
        if let Some(hit) = moller_trumbore_intersect(
            ray_origin,
            ray_dir,
            Vec3::from_array(tri.v0),
            Vec3::from_array(tri.v1),
            Vec3::from_array(tri.v2),
        ) {
            if hit.t < closest_t {
                closest_t = hit.t;
                closest = Some(hit.t);
            }
        }
    }

    closest
}
