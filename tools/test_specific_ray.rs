use glam::Vec3;
use ray_tracer::math::{world_to_cell, intersect_aabb};

fn main() {
    // New MISS ray
    let origin = Vec3::new(0.00, 5.00, 0.00);
    let direction = Vec3::new(-0.636, 0.475, 0.608).normalize();

    println!("\n=== ANALYZING MISS RAY ===");
    println!("Origin: ({:.2}, {:.2}, {:.2})", origin.x, origin.y, origin.z);
    println!("Direction: ({:.3}, {:.3}, {:.3})", direction.x, direction.y, direction.z);

    // Calculate where ray would hit west wall at x=-50
    let t_west_wall = (-50.0 - origin.x) / direction.x;
    let hit_point = origin + direction * t_west_wall;
    println!("\nExpected hit at west wall (x=-50):");
    println!("  t = {:.2}", t_west_wall);
    println!("  Hit point: ({:.2}, {:.2}, {:.2})", hit_point.x, hit_point.y, hit_point.z);

    // Check which wall box this would be
    // Wall structure: layer = y / 2.2, segment = (z - (-50)) / 2.2
    let box_size = 2.0;
    let spacing = 0.2;
    let stride = box_size + spacing; // 2.2

    let layer = (hit_point.y / stride).floor() as i32;
    let segment = ((hit_point.z - (-50.0)) / stride).floor() as i32;

    println!("\nWest wall box:");
    println!("  Layer: {} (y: {:.2} to {:.2})", layer, layer as f32 * stride, (layer as f32 + 1.0) * stride);
    println!("  Segment: {} (z: {:.2} to {:.2})", segment, -50.0 + segment as f32 * stride, -50.0 + (segment as f32 + 1.0) * stride);

    // Calculate actual box bounds
    let y_min = layer as f32 * stride;
    let y_max = y_min + box_size;
    let z_min = -50.0 + segment as f32 * stride;
    let z_max = z_min + box_size;

    let box_min = Vec3::new(-52.0, y_min, z_min);
    let box_max = Vec3::new(-50.0, y_max, z_max);

    println!("\nActual wall box bounds:");
    println!("  Min: ({:.2}, {:.2}, {:.2})", box_min.x, box_min.y, box_min.z);
    println!("  Max: ({:.2}, {:.2}, {:.2})", box_max.x, box_max.y, box_max.z);

    // Test direct intersection
    let t = intersect_aabb(origin, direction, box_min, box_max);
    println!("\nDirect AABB intersection test:");
    if t > 0.0 {
        let actual_hit = origin + direction * t;
        println!("  HIT at t={:.2}", t);
        println!("  Hit position: ({:.2}, {:.2}, {:.2})", actual_hit.x, actual_hit.y, actual_hit.z);
    } else {
        println!("  MISS (t={})", t);
    }

    // Grid analysis
    let bounds_min = Vec3::new(-201.0, -2.0, -201.0);
    let bounds_max = Vec3::new(201.0, 49.2, 201.0);
    let cell_size = 16.0;
    let grid_size = (
        ((bounds_max.x - bounds_min.x) / cell_size).ceil() as u32,
        ((bounds_max.y - bounds_min.y) / cell_size).ceil() as u32,
        ((bounds_max.z - bounds_min.z) / cell_size).ceil() as u32,
    );

    println!("\n=== GRID TRAVERSAL ===");
    println!("Grid: {}x{}x{}, cell_size={}", grid_size.0, grid_size.1, grid_size.2, cell_size);

    let start_cell = world_to_cell(origin, bounds_min, cell_size);
    let end_cell = world_to_cell(hit_point, bounds_min, cell_size);

    println!("Start cell: ({}, {}, {})", start_cell.0, start_cell.1, start_cell.2);
    println!("Expected end cell: ({}, {}, {})", end_cell.0, end_cell.1, end_cell.2);

    // Calculate cell of the wall box
    let box_center = (box_min + box_max) * 0.5;
    let box_cell = world_to_cell(box_center, bounds_min, cell_size);
    println!("Wall box center cell: ({}, {}, {})", box_cell.0, box_cell.1, box_cell.2);

    // DDA simulation
    println!("\n--- DDA Simulation ---");
    let step = (
        if direction.x >= 0.0 { 1 } else { -1 },
        if direction.y >= 0.0 { 1 } else { -1 },
        if direction.z >= 0.0 { 1 } else { -1 },
    );

    let cell_pos_world = bounds_min + Vec3::new(
        start_cell.0 as f32 * cell_size,
        start_cell.1 as f32 * cell_size,
        start_cell.2 as f32 * cell_size,
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

    let mut t_max = (next_boundary - origin) / direction;
    t_max = t_max.max(Vec3::splat(0.00001));

    println!("Step: ({}, {}, {})", step.0, step.1, step.2);
    println!("t_delta: ({:.3}, {:.3}, {:.3})", t_delta.x, t_delta.y, t_delta.z);
    println!("t_max initial: ({:.3}, {:.3}, {:.3})", t_max.x, t_max.y, t_max.z);

    let mut current = start_cell;

    for i in 0..30 {
        println!("Step {}: cell=({}, {}, {}), t_max=({:.3}, {:.3}, {:.3})",
            i, current.0, current.1, current.2, t_max.x, t_max.y, t_max.z);

        // Check if we reached the wall box cell
        if current == box_cell {
            println!("  -> REACHED WALL BOX CELL!");
        }

        // Check if we've gone past the expected hit distance
        let min_t = t_max.x.min(t_max.y).min(t_max.z);
        if min_t > t_west_wall {
            println!("  -> Would have hit wall before next cell boundary");
            break;
        }

        // Check bounds
        if current.0 < 0 || current.1 < 0 || current.2 < 0 ||
           current.0 >= grid_size.0 as i32 || current.1 >= grid_size.1 as i32 || current.2 >= grid_size.2 as i32 {
            println!("  -> OUT OF BOUNDS!");
            break;
        }

        // Step
        if t_max.x < t_max.y && t_max.x < t_max.z {
            let next_x = current.0 + step.0;
            if next_x < 0 || next_x >= grid_size.0 as i32 {
                println!("  -> Next X out of bounds, breaking");
                break;
            }
            current.0 = next_x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            let next_y = current.1 + step.1;
            if next_y < 0 || next_y >= grid_size.1 as i32 {
                println!("  -> Next Y out of bounds, breaking");
                break;
            }
            current.1 = next_y;
            t_max.y += t_delta.y;
        } else {
            let next_z = current.2 + step.2;
            if next_z < 0 || next_z >= grid_size.2 as i32 {
                println!("  -> Next Z out of bounds, breaking");
                break;
            }
            current.2 = next_z;
            t_max.z += t_delta.z;
        }
    }
}
