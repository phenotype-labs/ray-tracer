use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ray_tracer::core::sphere::SphereData;
use ray_tracer::math::intersect_aabb;
use glam::{Vec3, Mat4};
use std::f32::consts::PI;

#[cfg(target_os = "macos")]
use std::process::Command;

/// Get current process memory usage (macOS)
#[cfg(target_os = "macos")]
fn get_memory_usage() -> (usize, usize) {
    let pid = std::process::id();
    let output = Command::new("ps")
        .args(&["-o", "rss,vsz", "-p", &pid.to_string()])
        .output()
        .ok();

    if let Some(output) = output {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = output_str.lines().nth(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let rss = parts[0].parse::<usize>().unwrap_or(0) * 1024; // Convert KB to bytes
                let vsz = parts[1].parse::<usize>().unwrap_or(0) * 1024;
                return (rss, vsz);
            }
        }
    }
    (0, 0)
}

#[cfg(not(target_os = "macos"))]
fn get_memory_usage() -> (usize, usize) {
    (0, 0)
}

/// Generate random unit vector for ray directions
fn random_unit_vector(seed: u32) -> Vec3 {
    let theta = (seed as f32 * 0.123456) % (2.0 * PI);
    let phi = (seed as f32 * 0.789012) % PI;
    Vec3::new(
        phi.sin() * theta.cos(),
        phi.sin() * theta.sin(),
        phi.cos(),
    )
}

/// Benchmark: Single sphere intersection (hit case)
fn bench_sphere_intersection_hit(c: &mut Criterion) {
    let sphere = SphereData::new(Vec3::new(0.0, 0.0, -5.0), 1.0, [1.0, 0.0, 0.0]);
    let origin = Vec3::ZERO;
    let direction = Vec3::new(0.0, 0.0, -1.0);

    c.bench_function("sphere_intersection_hit", |b| {
        b.iter(|| {
            black_box(sphere.intersect(black_box(origin), black_box(direction)))
        })
    });
}

/// Benchmark: Single AABB intersection (hit case)
fn bench_aabb_intersection_hit(c: &mut Criterion) {
    let min = Vec3::new(-1.0, -1.0, -6.0);
    let max = Vec3::new(1.0, 1.0, -4.0);
    let origin = Vec3::ZERO;
    let direction = Vec3::new(0.0, 0.0, -1.0);

    c.bench_function("aabb_intersection_hit", |b| {
        b.iter(|| {
            black_box(intersect_aabb(
                black_box(origin),
                black_box(direction),
                black_box(min),
                black_box(max),
            ))
        })
    });
}

/// Benchmark: Single sphere intersection (miss case)
fn bench_sphere_intersection_miss(c: &mut Criterion) {
    let sphere = SphereData::new(Vec3::new(10.0, 10.0, -5.0), 1.0, [1.0, 0.0, 0.0]);
    let origin = Vec3::ZERO;
    let direction = Vec3::new(0.0, 0.0, -1.0);

    c.bench_function("sphere_intersection_miss", |b| {
        b.iter(|| {
            black_box(sphere.intersect(black_box(origin), black_box(direction)))
        })
    });
}

/// Benchmark: Single AABB intersection (miss case)
fn bench_aabb_intersection_miss(c: &mut Criterion) {
    let min = Vec3::new(10.0, 10.0, -6.0);
    let max = Vec3::new(12.0, 12.0, -4.0);
    let origin = Vec3::ZERO;
    let direction = Vec3::new(0.0, 0.0, -1.0);

    c.bench_function("aabb_intersection_miss", |b| {
        b.iter(|| {
            black_box(intersect_aabb(
                black_box(origin),
                black_box(direction),
                black_box(min),
                black_box(max),
            ))
        })
    });
}

/// Benchmark: Particle system simulation (1000 spheres, random rays)
fn bench_particle_system_spheres(c: &mut Criterion) {
    let mut group = c.benchmark_group("particle_system");

    for count in [100, 1000, 10000].iter() {
        let spheres: Vec<SphereData> = (0..*count)
            .map(|i| {
                let x = ((i as f32 * 0.1) % 20.0) - 10.0;
                let y = ((i as f32 * 0.2) % 20.0) - 10.0;
                let z = -((i as f32 * 0.3) % 50.0) - 10.0;
                SphereData::new(Vec3::new(x, y, z), 0.5, [1.0, 1.0, 1.0])
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("spheres", count), count, |b, _| {
            b.iter(|| {
                let mut hit_count = 0;
                for i in 0..100 {
                    let dir = random_unit_vector(i);
                    for sphere in &spheres {
                        if sphere.intersect(Vec3::ZERO, dir).is_some() {
                            hit_count += 1;
                        }
                    }
                }
                black_box(hit_count)
            })
        });
    }

    group.finish();
}

/// Benchmark: Particle system with AABBs (1000 boxes, random rays)
fn bench_particle_system_aabbs(c: &mut Criterion) {
    let mut group = c.benchmark_group("particle_system");

    for count in [100, 1000, 10000].iter() {
        let aabbs: Vec<(Vec3, Vec3)> = (0..*count)
            .map(|i| {
                let x = ((i as f32 * 0.1) % 20.0) - 10.0;
                let y = ((i as f32 * 0.2) % 20.0) - 10.0;
                let z = -((i as f32 * 0.3) % 50.0) - 10.0;
                let center = Vec3::new(x, y, z);
                let half_size = Vec3::splat(0.5);
                (center - half_size, center + half_size)
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("aabbs", count), count, |b, _| {
            b.iter(|| {
                let mut hit_count = 0;
                for i in 0..100 {
                    let dir = random_unit_vector(i);
                    for (min, max) in &aabbs {
                        if intersect_aabb(Vec3::ZERO, dir, *min, *max) > 0.0 {
                            hit_count += 1;
                        }
                    }
                }
                black_box(hit_count)
            })
        });
    }

    group.finish();
}

/// Benchmark: Rotation invariance - spheres don't need recomputation
fn bench_rotation_spheres(c: &mut Criterion) {
    let spheres: Vec<SphereData> = (0..1000)
        .map(|i| {
            let x = ((i as f32 * 0.1) % 20.0) - 10.0;
            let y = ((i as f32 * 0.2) % 20.0) - 10.0;
            let z = -((i as f32 * 0.3) % 50.0) - 10.0;
            SphereData::new(Vec3::new(x, y, z), 1.0, [1.0, 1.0, 1.0])
        })
        .collect();

    c.bench_function("rotation_spheres_no_recompute", |b| {
        b.iter(|| {
            // Sphere bounds don't change under rotation
            let mut hit_count = 0;
            let dir = Vec3::new(0.0, 0.0, -1.0);
            for sphere in &spheres {
                if sphere.intersect(Vec3::ZERO, dir).is_some() {
                    hit_count += 1;
                }
            }
            black_box(hit_count)
        })
    });
}

/// Benchmark: Rotation invariance - AABBs need recomputation after rotation
fn bench_rotation_aabbs(c: &mut Criterion) {
    let initial_aabbs: Vec<(Vec3, Vec3)> = (0..1000)
        .map(|i| {
            let x = ((i as f32 * 0.1) % 20.0) - 10.0;
            let y = ((i as f32 * 0.2) % 20.0) - 10.0;
            let z = -((i as f32 * 0.3) % 50.0) - 10.0;
            let center = Vec3::new(x, y, z);
            let half_size = Vec3::splat(1.0);
            (center - half_size, center + half_size)
        })
        .collect();

    c.bench_function("rotation_aabbs_with_recompute", |b| {
        b.iter(|| {
            // AABB bounds change under rotation - simulate recomputation
            let rotation = Mat4::from_rotation_y(0.1);
            let mut rotated_aabbs = Vec::with_capacity(1000);

            for (min, max) in &initial_aabbs {
                // Recompute AABB after rotation (8 corner transformation)
                let corners = [
                    Vec3::new(min.x, min.y, min.z),
                    Vec3::new(min.x, min.y, max.z),
                    Vec3::new(min.x, max.y, min.z),
                    Vec3::new(min.x, max.y, max.z),
                    Vec3::new(max.x, min.y, min.z),
                    Vec3::new(max.x, min.y, max.z),
                    Vec3::new(max.x, max.y, min.z),
                    Vec3::new(max.x, max.y, max.z),
                ];

                let transformed: Vec<Vec3> = corners
                    .iter()
                    .map(|&c| rotation.transform_point3(c))
                    .collect();

                let new_min = transformed.iter().fold(
                    Vec3::splat(f32::MAX),
                    |acc, &v| acc.min(v)
                );
                let new_max = transformed.iter().fold(
                    Vec3::splat(f32::MIN),
                    |acc, &v| acc.max(v)
                );

                rotated_aabbs.push((new_min, new_max));
            }

            let mut hit_count = 0;
            let dir = Vec3::new(0.0, 0.0, -1.0);
            for (min, max) in &rotated_aabbs {
                if intersect_aabb(Vec3::ZERO, dir, *min, *max) > 0.0 {
                    hit_count += 1;
                }
            }
            black_box(hit_count)
        })
    });
}

/// Benchmark: Thin geometry (plane-like) - worst case for spheres
fn bench_thin_geometry(c: &mut Criterion) {
    let mut group = c.benchmark_group("thin_geometry");

    // Sphere bounding a thin plane (very wasteful)
    let sphere = SphereData::new(Vec3::new(0.0, 0.0, -5.0), 5.0, [1.0, 1.0, 1.0]);

    // AABB bounding the same plane (much tighter)
    let aabb_min = Vec3::new(-5.0, -0.1, -5.0);
    let aabb_max = Vec3::new(5.0, 0.1, -5.0);

    group.bench_function("sphere_thin_plane", |b| {
        b.iter(|| {
            let mut hit_count = 0;
            for i in 0..1000 {
                let dir = random_unit_vector(i);
                if sphere.intersect(Vec3::ZERO, dir).is_some() {
                    hit_count += 1;
                }
            }
            black_box(hit_count)
        })
    });

    group.bench_function("aabb_thin_plane", |b| {
        b.iter(|| {
            let mut hit_count = 0;
            for i in 0..1000 {
                let dir = random_unit_vector(i);
                if intersect_aabb(Vec3::ZERO, dir, aabb_min, aabb_max) > 0.0 {
                    hit_count += 1;
                }
            }
            black_box(hit_count)
        })
    });

    group.finish();
}

/// Benchmark: Memory access patterns
fn bench_cache_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_efficiency");

    // Spheres: 4 floats (center.x, center.y, center.z, radius)
    let spheres: Vec<SphereData> = (0..10000)
        .map(|i| {
            SphereData::new(
                Vec3::new(i as f32 * 0.1, i as f32 * 0.2, -(i as f32 * 0.3)),
                1.0,
                [1.0, 1.0, 1.0]
            )
        })
        .collect();

    // AABBs: 6 floats (min.xyz, max.xyz)
    let aabbs: Vec<(Vec3, Vec3)> = (0..10000)
        .map(|i| {
            let center = Vec3::new(i as f32 * 0.1, i as f32 * 0.2, -(i as f32 * 0.3));
            let half = Vec3::splat(1.0);
            (center - half, center + half)
        })
        .collect();

    println!("\n=== Memory Footprint ===");
    println!("Sphere size: {} bytes", std::mem::size_of::<SphereData>());
    println!("AABB size: {} bytes", std::mem::size_of::<(Vec3, Vec3)>());
    println!("10k Spheres: {} KB", (spheres.len() * std::mem::size_of::<SphereData>()) / 1024);
    println!("10k AABBs: {} KB", (aabbs.len() * std::mem::size_of::<(Vec3, Vec3)>()) / 1024);

    group.bench_function("sphere_sequential_access", |b| {
        b.iter(|| {
            let dir = Vec3::new(0.0, 0.0, -1.0);
            let mut hit_count = 0;
            for sphere in &spheres {
                if sphere.intersect(Vec3::ZERO, dir).is_some() {
                    hit_count += 1;
                }
            }
            black_box(hit_count)
        })
    });

    group.bench_function("aabb_sequential_access", |b| {
        b.iter(|| {
            let dir = Vec3::new(0.0, 0.0, -1.0);
            let mut hit_count = 0;
            for (min, max) in &aabbs {
                if intersect_aabb(Vec3::ZERO, dir, *min, *max) > 0.0 {
                    hit_count += 1;
                }
            }
            black_box(hit_count)
        })
    });

    group.finish();
}

/// Benchmark: Memory allocation patterns
fn bench_memory_usage(c: &mut Criterion) {
    println!("\n=== Memory Usage Analysis ===");

    // Test different object counts
    for &count in &[1000, 10000, 100000] {
        println!("\n--- {} Objects ---", count);

        // Measure sphere allocation
        let (rss_before, _) = get_memory_usage();
        let spheres: Vec<SphereData> = (0..count)
            .map(|i| {
                SphereData::new(
                    Vec3::new(i as f32 * 0.1, i as f32 * 0.2, -(i as f32 * 0.3)),
                    1.0,
                    [1.0, 1.0, 1.0]
                )
            })
            .collect();
        let (rss_after_spheres, _) = get_memory_usage();

        let sphere_mem_actual = rss_after_spheres.saturating_sub(rss_before);
        let sphere_mem_theoretical = count * std::mem::size_of::<SphereData>();

        println!("Spheres:");
        println!("  Theoretical: {} KB ({} bytes each)",
            sphere_mem_theoretical / 1024, std::mem::size_of::<SphereData>());
        if sphere_mem_actual > 0 {
            println!("  Actual RSS: {} KB (includes Vec overhead + allocator metadata)",
                sphere_mem_actual / 1024);
        }

        drop(spheres);

        // Measure AABB allocation
        let (rss_before, _) = get_memory_usage();
        let aabbs: Vec<(Vec3, Vec3)> = (0..count)
            .map(|i| {
                let center = Vec3::new(i as f32 * 0.1, i as f32 * 0.2, -(i as f32 * 0.3));
                let half = Vec3::splat(1.0);
                (center - half, center + half)
            })
            .collect();
        let (rss_after_aabbs, _) = get_memory_usage();

        let aabb_mem_actual = rss_after_aabbs.saturating_sub(rss_before);
        let aabb_mem_theoretical = count * std::mem::size_of::<(Vec3, Vec3)>();

        println!("AABBs:");
        println!("  Theoretical: {} KB ({} bytes each)",
            aabb_mem_theoretical / 1024, std::mem::size_of::<(Vec3, Vec3)>());
        if aabb_mem_actual > 0 {
            println!("  Actual RSS: {} KB (includes Vec overhead + allocator metadata)",
                aabb_mem_actual / 1024);
        }

        println!("Memory Efficiency:");
        println!("  Sphere/AABB ratio: {:.2}x",
            sphere_mem_theoretical as f32 / aabb_mem_theoretical as f32);

        drop(aabbs);
    }

    // Benchmark memory access patterns
    let mut group = c.benchmark_group("memory_bandwidth");

    let sphere_count = 50000;
    let spheres: Vec<SphereData> = (0..sphere_count)
        .map(|i| {
            SphereData::new(
                Vec3::new(i as f32 * 0.1, i as f32 * 0.2, -(i as f32 * 0.3)),
                1.0,
                [1.0, 1.0, 1.0]
            )
        })
        .collect();

    let aabbs: Vec<(Vec3, Vec3)> = (0..sphere_count)
        .map(|i| {
            let center = Vec3::new(i as f32 * 0.1, i as f32 * 0.2, -(i as f32 * 0.3));
            let half = Vec3::splat(1.0);
            (center - half, center + half)
        })
        .collect();

    println!("\n=== Memory Bandwidth Test (50k objects) ===");
    println!("Testing sequential access patterns...");

    group.bench_function("sphere_bandwidth", |b| {
        b.iter(|| {
            let dir = Vec3::new(0.0, 0.0, -1.0);
            let mut sum = 0.0f32;
            for sphere in &spheres {
                if let Some(t) = sphere.intersect(Vec3::ZERO, dir) {
                    sum += t;
                }
            }
            black_box(sum)
        })
    });

    group.bench_function("aabb_bandwidth", |b| {
        b.iter(|| {
            let dir = Vec3::new(0.0, 0.0, -1.0);
            let mut sum = 0.0f32;
            for (min, max) in &aabbs {
                let t = intersect_aabb(Vec3::ZERO, dir, *min, *max);
                if t > 0.0 {
                    sum += t;
                }
            }
            black_box(sum)
        })
    });

    group.finish();

    println!("\n=== CPU Cache Line Analysis ===");
    println!("Cache line size (typical): 64 bytes");
    println!("Spheres per cache line: {}", 64 / std::mem::size_of::<SphereData>());
    println!("AABBs per cache line: {}", 64 / std::mem::size_of::<(Vec3, Vec3)>());
    println!("\nCache efficiency: Smaller objects = more per cache line = fewer cache misses");
}

criterion_group!(
    benches,
    bench_sphere_intersection_hit,
    bench_aabb_intersection_hit,
    bench_sphere_intersection_miss,
    bench_aabb_intersection_miss,
    bench_particle_system_spheres,
    bench_particle_system_aabbs,
    bench_rotation_spheres,
    bench_rotation_aabbs,
    bench_thin_geometry,
    bench_cache_efficiency,
    bench_memory_usage,
);

criterion_main!(benches);
