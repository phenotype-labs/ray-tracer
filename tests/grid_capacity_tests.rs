use ray_tracer::grid::{HierarchicalGrid, MAX_OBJECTS_PER_CELL};
use ray_tracer::types::BoxData;
use glam::Vec3;

/// Test that the grid doesn't silently drop objects when cells are at capacity
#[test]
fn test_grid_capacity_no_drops() {
    // Create a scene where many small boxes are placed in the same cell
    let mut boxes = Vec::new();

    // Place 100 small boxes in the same region (will map to same cell)
    for i in 0..100 {
        let offset = (i as f32) * 0.1; // Small offset to keep them in same cell
        boxes.push(BoxData::new(
            [offset, 0.0, 0.0],
            [offset + 0.05, 0.05, 0.05],
            [1.0, 0.0, 0.0],
        ));
    }

    let grid = HierarchicalGrid::build(&boxes);

    // Count total objects in all fine cells
    let total_objects_in_grid: usize = grid
        .fine_level
        .cells
        .iter()
        .map(|cell| cell.len())
        .sum();

    // All 100 objects should be in the grid (none dropped)
    assert_eq!(
        total_objects_in_grid, 100,
        "Expected all 100 objects in grid, but found {}. Objects were dropped!",
        total_objects_in_grid
    );
}

/// Test that cells can hold more than the old 64-object limit
#[test]
fn test_cell_capacity_exceeds_old_limit() {
    let old_limit = 64;
    let mut boxes = Vec::new();

    // Create more boxes than the old limit, all in same cell
    for i in 0..100 {
        let offset = (i as f32) * 0.1;
        boxes.push(BoxData::new(
            [offset, 0.0, 0.0],
            [offset + 0.05, 0.05, 0.05],
            [1.0, 0.0, 0.0],
        ));
    }

    let grid = HierarchicalGrid::build(&boxes);

    // Find the cell with the most objects
    let max_objects = grid
        .fine_level
        .cells
        .iter()
        .map(|cell| cell.len())
        .max()
        .unwrap_or(0);

    assert!(
        max_objects > old_limit,
        "Max objects in a cell ({}) should exceed old limit ({}), but didn't. This means the capacity increase isn't being used.",
        max_objects, old_limit
    );
}

/// Test that MAX_OBJECTS_PER_CELL is large enough for typical dense scenes
#[test]
fn test_max_objects_per_cell_sufficient() {
    // The walls scene has 4204 boxes, and diagnostics showed max 80 objects in a cell
    // MAX_OBJECTS_PER_CELL should be at least 3-4x the maximum observed
    let expected_max_in_dense_scene = 80;
    let safety_margin = 3;

    assert!(
        MAX_OBJECTS_PER_CELL >= expected_max_in_dense_scene * safety_margin,
        "MAX_OBJECTS_PER_CELL ({}) should be at least {}x the observed max ({}), but is only {}x",
        MAX_OBJECTS_PER_CELL,
        safety_margin,
        expected_max_in_dense_scene,
        MAX_OBJECTS_PER_CELL / expected_max_in_dense_scene
    );
}

/// Test that grid building warns when capacity is exceeded (if we ever hit it)
#[test]
fn test_grid_capacity_warning() {
    // This test verifies the warning system works if capacity is ever exceeded
    // We can't easily test stderr output in unit tests, but we can verify
    // that the data structure behavior is correct

    let mut boxes = Vec::new();

    // Create just enough boxes to potentially reach capacity
    // Place them all in a tight cluster
    for i in 0..MAX_OBJECTS_PER_CELL + 10 {
        let offset = (i as f32) * 0.01; // Very small offsets
        boxes.push(BoxData::new(
            [offset, 0.0, 0.0],
            [offset + 0.005, 0.005, 0.005],
            [1.0, 0.0, 0.0],
        ));
    }

    let grid = HierarchicalGrid::build(&boxes);

    // Count objects in the most populated cell
    let max_objects = grid
        .fine_level
        .cells
        .iter()
        .map(|cell| cell.len())
        .max()
        .unwrap_or(0);

    // Should not exceed MAX_OBJECTS_PER_CELL (extra objects dropped with warning)
    assert!(
        max_objects <= MAX_OBJECTS_PER_CELL,
        "Cell has {} objects, exceeding MAX_OBJECTS_PER_CELL ({})",
        max_objects, MAX_OBJECTS_PER_CELL
    );
}

/// Test walls scene specific case: verify objects in cell (9, 2, 15) are present
#[test]
fn test_walls_scene_cell_population() {
    // Simulate a simplified walls scene with boxes at layer 19, segment 44
    let mut boxes = Vec::new();

    // Add ground
    boxes.push(BoxData::new(
        [-200.0, -1.0, -200.0],
        [200.0, -0.99, 200.0],
        [0.15, 0.15, 0.15],
    ));

    // Add wall boxes around the critical cell (9, 2, 15)
    // Cell (9, 2, 15) in a 16-unit grid with bounds (-201, -2, -201):
    // X: [-201 + 9*16, -201 + 10*16] = [-57, -41]
    // Y: [-2 + 2*16, -2 + 3*16] = [30, 46]
    // Z: [-201 + 15*16, -201 + 16*16] = [39, 55]

    let stride = 2.2;
    for layer in 13..22 {  // Layers around y=30-46
        for segment in 40..50 {  // Segments around z=39-55
            let y_min = layer as f32 * stride;
            let y_max = y_min + 2.0;
            let z_min = -50.0 + segment as f32 * stride;
            let z_max = z_min + 2.0;

            // West wall
            boxes.push(BoxData::new(
                [-52.0, y_min, z_min],
                [-50.0, y_max, z_max],
                [1.0, 0.0, 0.0],
            ));
        }
    }

    let grid = HierarchicalGrid::build(&boxes);

    // Calculate cell (9, 2, 15) index
    let cell_idx = 9 + 2 * grid.fine_level.grid_size[0] + 15 * grid.fine_level.grid_size[0] * grid.fine_level.grid_size[1];
    let objects_in_critical_cell = grid.fine_level.cells[cell_idx].len();

    assert!(
        objects_in_critical_cell > 0,
        "Cell (9, 2, 15) should contain wall boxes, but has {} objects",
        objects_in_critical_cell
    );

    // Specifically check for the wall box at layer 19, segment 44
    // This box should be in the grid
    let layer_19_segment_44 = (-52.0, 41.8, 46.8); // Box min corner
    let mut found_target_box = false;

    for &obj_id in grid.fine_level.cells[cell_idx].iter() {
        let box_data = &boxes[obj_id as usize];
        let box_min = Vec3::from_array(box_data.min);

        // Check if this is approximately the layer 19, segment 44 box
        if (box_min.x - layer_19_segment_44.0).abs() < 0.1 &&
           (box_min.y - layer_19_segment_44.1).abs() < 0.1 &&
           (box_min.z - layer_19_segment_44.2).abs() < 0.1 {
            found_target_box = true;
            break;
        }
    }

    assert!(
        found_target_box,
        "Layer 19, segment 44 wall box not found in cell (9, 2, 15)"
    );
}

/// Test that grid stats are calculated correctly
#[test]
fn test_grid_stats_accuracy() {
    let mut boxes = Vec::new();

    // Create a known number of boxes in specific locations
    // 3 boxes in one cell, 2 in another, 1 in a third
    boxes.push(BoxData::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 0.0]));
    boxes.push(BoxData::new([0.5, 0.5, 0.5], [1.5, 1.5, 1.5], [0.0, 1.0, 0.0]));
    boxes.push(BoxData::new([0.7, 0.7, 0.7], [1.7, 1.7, 1.7], [0.0, 0.0, 1.0]));

    boxes.push(BoxData::new([20.0, 0.0, 0.0], [21.0, 1.0, 1.0], [1.0, 1.0, 0.0]));
    boxes.push(BoxData::new([20.5, 0.5, 0.5], [21.5, 1.5, 1.5], [1.0, 0.0, 1.0]));

    boxes.push(BoxData::new([40.0, 0.0, 0.0], [41.0, 1.0, 1.0], [0.0, 1.0, 1.0]));

    let grid = HierarchicalGrid::build(&boxes);

    // Count occupied cells
    let occupied_cells = grid
        .fine_level
        .cells
        .iter()
        .filter(|cell| !cell.is_empty())
        .count();

    // Should have exactly 3 occupied cells
    assert_eq!(
        occupied_cells, 3,
        "Expected 3 occupied cells, found {}",
        occupied_cells
    );

    // Find max objects in a cell
    let max_objects = grid
        .fine_level
        .cells
        .iter()
        .map(|cell| cell.len())
        .max()
        .unwrap_or(0);

    // Should be 3 (first cell has 3 boxes)
    assert_eq!(
        max_objects, 3,
        "Expected max 3 objects in a cell, found {}",
        max_objects
    );

    // Count total objects
    let total: usize = grid
        .fine_level
        .cells
        .iter()
        .map(|cell| cell.len())
        .sum();

    assert_eq!(
        total, 6,
        "Expected 6 total objects in grid, found {}",
        total
    );
}

/// Regression test for the specific MISS case reported by user
#[test]
fn test_user_reported_miss_case() {
    // User reported: Ray Origin (0.00, 5.00, 0.00), Direction (-0.636, 0.475, 0.608)
    // Should hit west wall at layer 19, segment 44

    let mut boxes = Vec::new();

    // Add the specific wall box that was missing
    let layer = 19;
    let segment = 44;
    let stride = 2.2;
    let y_min = layer as f32 * stride;  // 41.8
    let z_min = -50.0 + segment as f32 * stride;  // 46.8

    boxes.push(BoxData::new(
        [-52.0, y_min, z_min],
        [-50.0, y_min + 2.0, z_min + 2.0],
        [1.0, 0.0, 0.0],
    ));

    let grid = HierarchicalGrid::build(&boxes);

    // Verify the box was added to the grid
    let total_objects: usize = grid
        .fine_level
        .cells
        .iter()
        .map(|cell| cell.len())
        .sum();

    assert_eq!(
        total_objects, 1,
        "Wall box should be in grid, but found {} objects",
        total_objects
    );
}
