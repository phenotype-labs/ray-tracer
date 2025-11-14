use glam::Vec3;

fn world_to_cell(pos: Vec3, bounds_min: Vec3, cell_size: f32) -> (i32, i32, i32) {
    let rel_pos = pos - bounds_min;
    (
        (rel_pos.x / cell_size).floor() as i32,
        (rel_pos.y / cell_size).floor() as i32,
        (rel_pos.z / cell_size).floor() as i32,
    )
}

fn intersect_aabb(ray_origin: Vec3, ray_dir: Vec3, box_min: Vec3, box_max: Vec3) -> f32 {
    let t_min = (box_min - ray_origin) / ray_dir;
    let t_max = (box_max - ray_origin) / ray_dir;

    let t1 = t_min.min(t_max);
    let t2 = t_min.max(t_max);

    let t_near = t1.x.max(t1.y).max(t1.z);
    let t_far = t2.x.min(t2.y).min(t2.z);

    if t_near > t_far || t_far < 0.0 {
        return -1.0;
    }

    if t_near < 0.0 {
        t_far
    } else {
        t_near
    }
}

fn trace_ray_test(origin: Vec3, direction: Vec3, bounds_min: Vec3, bounds_max: Vec3, cell_size: f32, grid_size: (u32, u32, u32)) {
    println!("\n=== RAY TRACE TEST ===");
    println!("Ray Origin: ({:.2}, {:.2}, {:.2})", origin.x, origin.y, origin.z);
    println!("Ray Direction: ({:.3}, {:.3}, {:.3})", direction.x, direction.y, direction.z);
    println!("Grid bounds: ({:.2}, {:.2}, {:.2}) to ({:.2}, {:.2}, {:.2})",
        bounds_min.x, bounds_min.y, bounds_min.z,
        bounds_max.x, bounds_max.y, bounds_max.z);
    println!("Grid size: {}x{}x{}, cell size: {}", grid_size.0, grid_size.1, grid_size.2, cell_size);

    let mut ray_pos = origin;
    let mut t_offset = 0.0;

    if ray_pos.x < bounds_min.x || ray_pos.x > bounds_max.x ||
       ray_pos.y < bounds_min.y || ray_pos.y > bounds_max.y ||
       ray_pos.z < bounds_min.z || ray_pos.z > bounds_max.z {
        let t_entry = intersect_aabb(origin, direction, bounds_min, bounds_max);
        println!("Ray outside grid, t_entry = {:.2}", t_entry);
        if t_entry < 0.0 {
            println!("Ray misses grid entirely!");
            return;
        }
        t_offset = t_entry + 0.001;
        ray_pos = origin + direction * t_offset;
        println!("Entry point: ({:.2}, {:.2}, {:.2}) at t={:.2}", ray_pos.x, ray_pos.y, ray_pos.z, t_offset);
    }

    let current_cell = world_to_cell(ray_pos, bounds_min, cell_size);
    println!("Starting cell: ({}, {}, {})", current_cell.0, current_cell.1, current_cell.2);

    let step = (
        if direction.x >= 0.0 { 1 } else { -1 },
        if direction.y >= 0.0 { 1 } else { -1 },
        if direction.z >= 0.0 { 1 } else { -1 },
    );
    println!("Step direction: ({}, {}, {})", step.0, step.1, step.2);

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
        (cell_size / direction.x).abs(),
        (cell_size / direction.y).abs(),
        (cell_size / direction.z).abs(),
    );
    let mut t_max = t_offset + (next_boundary - ray_pos) / direction;
    t_max = t_max.max(Vec3::splat(t_offset + 0.00001));

    println!("t_delta: ({:.3}, {:.3}, {:.3})", t_delta.x, t_delta.y, t_delta.z);
    println!("t_max initial: ({:.3}, {:.3}, {:.3})", t_max.x, t_max.y, t_max.z);

    let mut current = current_cell;

    println!("\n--- DDA Traversal ---");
    for i in 0..50 {
        let current_u32 = (current.0 as u32, current.1 as u32, current.2 as u32);

        println!("Step {}: cell=({}, {}, {}) | as_u32=({}, {}, {}) | t_max=({:.3}, {:.3}, {:.3})",
            i, current.0, current.1, current.2,
            current_u32.0, current_u32.1, current_u32.2,
            t_max.x, t_max.y, t_max.z);

        if current.0 < 0 || current.1 < 0 || current.2 < 0 {
            println!("  -> NEGATIVE cell coordinate detected! (will wrap to huge u32)");
        }

        if current_u32.0 >= grid_size.0 || current_u32.1 >= grid_size.1 || current_u32.2 >= grid_size.2 {
            println!("  -> OUT OF BOUNDS! Breaking.");
            break;
        }

        // NEW LOGIC: Check bounds before stepping
        if t_max.x < t_max.y && t_max.x < t_max.z {
            let next_x = current.0 + step.0;
            println!("  -> Stepping X: {} -> {}", current.0, next_x);
            if next_x < 0 || next_x >= grid_size.0 as i32 {
                println!("     NEXT CELL OUT OF BOUNDS! Breaking.");
                break;
            }
            current.0 = next_x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            let next_y = current.1 + step.1;
            println!("  -> Stepping Y: {} -> {}", current.1, next_y);
            if next_y < 0 || next_y >= grid_size.1 as i32 {
                println!("     NEXT CELL OUT OF BOUNDS! Breaking.");
                break;
            }
            current.1 = next_y;
            t_max.y += t_delta.y;
        } else {
            let next_z = current.2 + step.2;
            println!("  -> Stepping Z: {} -> {}", current.2, next_z);
            if next_z < 0 || next_z >= grid_size.2 as i32 {
                println!("     NEXT CELL OUT OF BOUNDS! Breaking.");
                break;
            }
            current.2 = next_z;
            t_max.z += t_delta.z;
        }
    }

    println!("=== END TRACE ===\n");
}

fn main() {
    let origin = Vec3::new(-28.62, 28.00, -4.03);
    let direction = Vec3::new(-0.867, 0.030, 0.497).normalize();
    let bounds_min = Vec3::new(-201.0, -2.0, -201.0);
    let bounds_max = Vec3::new(201.0, 49.2, 201.0);
    let cell_size = 16.0;
    let grid_size = (27, 5, 27);

    println!("Testing MISS ray (should hit wall at x=-50):");
    trace_ray_test(origin, direction, bounds_min, bounds_max, cell_size, grid_size);

    let origin2 = Vec3::new(-28.62, 28.00, -4.03);
    let direction2 = Vec3::new(-0.867, 0.030, 0.497).normalize();
    println!("\nTesting HIT ray:");
    trace_ray_test(origin2, direction2, bounds_min, bounds_max, cell_size, grid_size);
}
