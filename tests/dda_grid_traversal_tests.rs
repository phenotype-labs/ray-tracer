use glam::Vec3;
use ray_tracer::math::{world_to_cell, intersect_aabb};

/// DDA grid traversal - returns number of cells visited and whether a box was hit
fn dda_traverse_grid(
    ray_origin: Vec3,
    ray_dir: Vec3,
    bounds_min: Vec3,
    bounds_max: Vec3,
    cell_size: f32,
    grid_size: (u32, u32, u32),
    boxes: &[(Vec3, Vec3)], // List of (box_min, box_max) pairs
    max_steps: u32,
) -> (u32, bool, Option<f32>) {
    let mut ray_pos = ray_origin;
    let mut t_offset = 0.0;

    // Check if ray starts outside grid
    if ray_pos.x < bounds_min.x || ray_pos.x > bounds_max.x
        || ray_pos.y < bounds_min.y || ray_pos.y > bounds_max.y
        || ray_pos.z < bounds_min.z || ray_pos.z > bounds_max.z
    {
        let t_entry = intersect_aabb(ray_origin, ray_dir, bounds_min, bounds_max);
        if t_entry < 0.0 {
            return (0, false, None);
        }
        t_offset = t_entry + 0.001;
        ray_pos = ray_origin + ray_dir * t_offset;
    }

    let current_cell = world_to_cell(ray_pos, bounds_min, cell_size);

    let step = (
        if ray_dir.x >= 0.0 { 1 } else { -1 },
        if ray_dir.y >= 0.0 { 1 } else { -1 },
        if ray_dir.z >= 0.0 { 1 } else { -1 },
    );

    let cell_pos_world = bounds_min + Vec3::new(
        current_cell.0 as f32 * cell_size,
        current_cell.1 as f32 * cell_size,
        current_cell.2 as f32 * cell_size,
    );

    let next_boundary = cell_pos_world + Vec3::new(
        if step.0 > 0 { cell_size } else { 0.0 },
        if step.1 > 0 { cell_size } else { 0.0 },
        if step.2 > 0 { cell_size } else { 0.0 },
    );

    let t_delta = Vec3::new(
        (cell_size / ray_dir.x).abs(),
        (cell_size / ray_dir.y).abs(),
        (cell_size / ray_dir.z).abs(),
    );

    let mut t_max = t_offset + (next_boundary - ray_pos) / ray_dir;
    t_max = t_max.max(Vec3::splat(t_offset + 0.00001));

    let mut current = current_cell;
    let mut steps = 0u32;

    for _ in 0..max_steps {
        let current_u32 = (current.0 as u32, current.1 as u32, current.2 as u32);

        // Out of bounds check
        if current.0 < 0 || current.1 < 0 || current.2 < 0
            || current_u32.0 >= grid_size.0
            || current_u32.1 >= grid_size.1
            || current_u32.2 >= grid_size.2
        {
            break;
        }

        steps += 1;

        // Check if any box intersects current cell
        let cell_min = bounds_min + Vec3::new(
            current.0 as f32 * cell_size,
            current.1 as f32 * cell_size,
            current.2 as f32 * cell_size,
        );
        let cell_max = cell_min + Vec3::splat(cell_size);

        for (box_min, box_max) in boxes {
            // Check if box overlaps with current cell
            if box_max.x >= cell_min.x && box_min.x <= cell_max.x
                && box_max.y >= cell_min.y && box_min.y <= cell_max.y
                && box_max.z >= cell_min.z && box_min.z <= cell_max.z
            {
                let t = intersect_aabb(ray_origin, ray_dir, *box_min, *box_max);
                if t > 0.0 {
                    return (steps, true, Some(t));
                }
            }
        }

        // DDA step
        if t_max.x < t_max.y && t_max.x < t_max.z {
            let next_x = current.0 + step.0;
            if next_x < 0 || next_x >= grid_size.0 as i32 {
                break;
            }
            current.0 = next_x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            let next_y = current.1 + step.1;
            if next_y < 0 || next_y >= grid_size.1 as i32 {
                break;
            }
            current.1 = next_y;
            t_max.y += t_delta.y;
        } else {
            let next_z = current.2 + step.2;
            if next_z < 0 || next_z >= grid_size.2 as i32 {
                break;
            }
            current.2 = next_z;
            t_max.z += t_delta.z;
        }
    }

    (steps, false, None)
}

#[cfg(test)]
mod walls_scene_tests {
    use super::*;

    #[test]
    fn test_west_wall_hit_pixel_722_131() {
        // Real-world case from walls scene - pixel (722, 131)
        // This ray SHOULD hit the west wall but currently MISSES after 2 steps

        let ray_origin = Vec3::new(-3.80, 18.10, 0.00);
        let ray_dir = Vec3::new(-0.684, 0.357, 0.636).normalize();

        // Walls scene uses these bounds (from scene.rs create_walls_scene)
        // Ground at y=-1, walls extend to y=50
        // Scene bounds should cover [-200, 200] in X/Z
        let bounds_min = Vec3::new(-201.0, -2.0, -201.0);
        let bounds_max = Vec3::new(201.0, 49.2, 201.0);
        let cell_size = 16.0;

        // Calculate grid size
        let extent = bounds_max - bounds_min;
        let grid_size = (
            (extent.x / cell_size).ceil() as u32,
            (extent.y / cell_size).ceil() as u32,
            (extent.z / cell_size).ceil() as u32,
        );

        // West wall box that should be hit:
        // Layer 19, Segment 42
        // X: [-52.0, -50.0] (west wall)
        // Y: [41.80, 43.80] (layer 19: 19 * (2.0 + 0.2) = 41.8)
        // Z: [42.40, 44.40] (segment 42: -50 + 42 * (2.0 + 0.2) = 42.4)
        let box_min = Vec3::new(-52.0, 41.8, 42.4);
        let box_max = Vec3::new(-50.0, 43.8, 44.4);
        let boxes = vec![(box_min, box_max)];

        let (steps, hit, t_hit) = dda_traverse_grid(
            ray_origin,
            ray_dir,
            bounds_min,
            bounds_max,
            cell_size,
            grid_size,
            &boxes,
            1000, // Generous max steps
        );

        // Expected hit point: (-50.0, 42.21, 42.96) at t≈67.54
        let expected_t = 67.54;
        let expected_hit_point = Vec3::new(-50.0, 42.21, 42.96);

        assert!(hit,
            "Ray should HIT the west wall box but got MISS. Steps taken: {}", steps);

        if let Some(t) = t_hit {
            assert!((t - expected_t).abs() < 1.0,
                "Hit distance should be ~{:.2}, got {:.2}", expected_t, t);

            let actual_hit_point = ray_origin + ray_dir * t;
            let distance_to_expected = (actual_hit_point - expected_hit_point).length();
            assert!(distance_to_expected < 1.0,
                "Hit point should be near ({:.2}, {:.2}, {:.2}), got ({:.2}, {:.2}, {:.2})",
                expected_hit_point.x, expected_hit_point.y, expected_hit_point.z,
                actual_hit_point.x, actual_hit_point.y, actual_hit_point.z);
        }

        // DDA should take ~8 steps:  (12,1,12) → (9,2,15) = 3X + 1Y + 3Z + initial = ~8 steps
        assert!(steps >= 7 && steps <= 10,
            "Should take ~8 DDA steps to reach wall (3X+1Y+3Z), but took {}", steps);
    }

    #[test]
    fn test_dda_long_distance_traversal() {
        // Test that DDA can traverse long distances through empty space
        let ray_origin = Vec3::new(0.0, 10.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0).normalize();

        let bounds_min = Vec3::new(-100.0, -10.0, -100.0);
        let bounds_max = Vec3::new(100.0, 50.0, 100.0);
        let cell_size = 16.0;

        let extent = bounds_max - bounds_min;
        let grid_size = (
            (extent.x / cell_size).ceil() as u32,
            (extent.y / cell_size).ceil() as u32,
            (extent.z / cell_size).ceil() as u32,
        );

        // Box far away at x=90
        let box_min = Vec3::new(90.0, 9.0, -1.0);
        let box_max = Vec3::new(92.0, 11.0, 1.0);
        let boxes = vec![(box_min, box_max)];

        let (steps, hit, t_hit) = dda_traverse_grid(
            ray_origin,
            ray_dir,
            bounds_min,
            bounds_max,
            cell_size,
            grid_size,
            &boxes,
            1000,
        );

        assert!(hit, "Should hit distant box at x=90");
        assert!(steps > 5, "Should take multiple DDA steps to reach distant box, got {}", steps);

        if let Some(t) = t_hit {
            assert!(t > 80.0 && t < 95.0, "Hit distance should be around 90, got {}", t);
        }
    }

    #[test]
    fn test_dda_does_not_exit_grid_prematurely() {
        // Test that DDA doesn't exit grid bounds incorrectly
        let ray_origin = Vec3::new(-5.0, 20.0, 0.0);
        let ray_dir = Vec3::new(-1.0, 0.1, 0.5).normalize();

        let bounds_min = Vec3::new(-200.0, -2.0, -200.0);
        let bounds_max = Vec3::new(200.0, 50.0, 200.0);
        let cell_size = 16.0;

        let extent = bounds_max - bounds_min;
        let grid_size = (
            (extent.x / cell_size).ceil() as u32,
            (extent.y / cell_size).ceil() as u32,
            (extent.z / cell_size).ceil() as u32,
        );

        // No boxes - just count steps
        let boxes = vec![];

        let (steps, _, _) = dda_traverse_grid(
            ray_origin,
            ray_dir,
            bounds_min,
            bounds_max,
            cell_size,
            grid_size,
            &boxes,
            1000,
        );

        // Should traverse many cells before exiting the large grid
        // DDA is optimized, so it won't traverse every cell individually
        assert!(steps >= 15,
            "Ray should traverse several cells in large grid before exiting, got {} steps", steps);
    }

    #[test]
    fn test_west_wall_direct_intersection() {
        // Simplified test: just check if ray intersects the box directly
        let ray_origin = Vec3::new(-3.80, 18.10, 0.00);
        let ray_dir = Vec3::new(-0.684, 0.357, 0.636).normalize();

        // West wall box
        let box_min = Vec3::new(-52.0, 41.8, 42.4);
        let box_max = Vec3::new(-50.0, 43.8, 44.4);

        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);

        assert!(t > 0.0,
            "Ray should intersect west wall box directly, got t={}", t);
        assert!((t - 67.54).abs() < 0.1,
            "Intersection distance should be ~67.54, got {}", t);
    }
}
