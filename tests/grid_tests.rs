use glam::Vec3;
use ray_tracer::grid::{CoarseGridLevel, FineGridLevel, FINEST_CELL_SIZE};
use ray_tracer::math::AABB;

#[cfg(test)]
mod coarse_grid_tests {
    use super::*;

    #[test]
    fn test_coarse_grid_creation() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };
        let cell_size = 25.0;

        let grid = CoarseGridLevel::new(&bounds, cell_size);

        assert_eq!(grid.cell_size, 25.0);
        assert!(grid.grid_size[0] >= 4, "Grid should have at least 4 cells in X");
        assert!(grid.grid_size[1] >= 4, "Grid should have at least 4 cells in Y");
        assert!(grid.grid_size[2] >= 4, "Grid should have at least 4 cells in Z");

        let total_cells = grid.grid_size[0] * grid.grid_size[1] * grid.grid_size[2];
        assert_eq!(grid.counts.len(), total_cells, "Should allocate correct number of cells");
    }

    #[test]
    fn test_coarse_grid_cell_index() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };
        let cell_size = 25.0;

        let grid = CoarseGridLevel::new(&bounds, cell_size);

        let idx_origin = grid.cell_index(0, 0, 0);
        assert_eq!(idx_origin, 0);

        let idx_x = grid.cell_index(1, 0, 0);
        assert_eq!(idx_x, 1);

        let idx_y = grid.cell_index(0, 1, 0);
        assert_eq!(idx_y, grid.grid_size[0]);

        let idx_z = grid.cell_index(0, 0, 1);
        assert_eq!(idx_z, grid.grid_size[0] * grid.grid_size[1]);
    }

    #[test]
    fn test_coarse_grid_increment_cell() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };
        let cell_size = 25.0;

        let mut grid = CoarseGridLevel::new(&bounds, cell_size);

        grid.increment_cell(0, 0, 0);
        grid.increment_cell(0, 0, 0);
        grid.increment_cell(1, 0, 0);

        let idx_origin = grid.cell_index(0, 0, 0);
        let idx_x = grid.cell_index(1, 0, 0);

        assert_eq!(grid.counts[idx_origin], 2);
        assert_eq!(grid.counts[idx_x], 1);
    }

    #[test]
    fn test_coarse_grid_max_count_255() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };
        let cell_size = 25.0;

        let mut grid = CoarseGridLevel::new(&bounds, cell_size);

        // Increment 300 times (more than u8::MAX)
        for _ in 0..300 {
            grid.increment_cell(0, 0, 0);
        }

        let idx_origin = grid.cell_index(0, 0, 0);
        assert_eq!(grid.counts[idx_origin], 255, "Count should be capped at 255");
    }
}

#[cfg(test)]
mod fine_grid_tests {
    use super::*;

    #[test]
    fn test_fine_grid_creation() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };

        let grid = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);

        assert_eq!(grid.cell_size, FINEST_CELL_SIZE);
        assert!(grid.grid_size[0] > 0, "Grid should have cells in X");
        assert!(grid.grid_size[1] > 0, "Grid should have cells in Y");
        assert!(grid.grid_size[2] > 0, "Grid should have cells in Z");

        let total_cells = grid.grid_size[0] * grid.grid_size[1] * grid.grid_size[2];
        assert_eq!(grid.cells.len(), total_cells, "Should allocate correct number of cells");
    }

    #[test]
    fn test_fine_grid_cell_index() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };

        let grid = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);

        let idx_origin = grid.cell_index(0, 0, 0);
        assert_eq!(idx_origin, 0);

        let idx_x = grid.cell_index(1, 0, 0);
        assert_eq!(idx_x, 1);

        let idx_y = grid.cell_index(0, 1, 0);
        assert_eq!(idx_y, grid.grid_size[0]);

        let idx_z = grid.cell_index(0, 0, 1);
        assert_eq!(idx_z, grid.grid_size[0] * grid.grid_size[1]);
    }

    #[test]
    fn test_fine_grid_add_object() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };

        let mut grid = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);

        grid.add_object(0, 0, 0, 42);
        grid.add_object(0, 0, 0, 99);
        grid.add_object(1, 0, 0, 7);

        let idx_origin = grid.cell_index(0, 0, 0);
        let idx_x = grid.cell_index(1, 0, 0);

        assert_eq!(grid.cells[idx_origin].len(), 2);
        assert_eq!(grid.cells[idx_origin][0], 42);
        assert_eq!(grid.cells[idx_origin][1], 99);
        assert_eq!(grid.cells[idx_x].len(), 1);
        assert_eq!(grid.cells[idx_x][0], 7);
    }

    #[test]
    fn test_fine_grid_max_objects_per_cell() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };

        let mut grid = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);

        // Add 100 objects (less than MAX_OBJECTS_PER_CELL which is 256)
        for i in 0..100 {
            grid.add_object(0, 0, 0, i);
        }

        let idx_origin = grid.cell_index(0, 0, 0);
        assert_eq!(
            grid.cells[idx_origin].len(),
            100,
            "Cell should hold all 100 objects (under the 256 limit)"
        );
    }

    #[test]
    fn test_fine_grid_different_cells() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 100.0, 100.0),
        };

        let mut grid = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);

        grid.add_object(0, 0, 0, 1);
        grid.add_object(1, 0, 0, 2);
        grid.add_object(0, 1, 0, 3);
        grid.add_object(0, 0, 1, 4);

        let idx_000 = grid.cell_index(0, 0, 0);
        let idx_100 = grid.cell_index(1, 0, 0);
        let idx_010 = grid.cell_index(0, 1, 0);
        let idx_001 = grid.cell_index(0, 0, 1);

        assert_eq!(grid.cells[idx_000][0], 1);
        assert_eq!(grid.cells[idx_100][0], 2);
        assert_eq!(grid.cells[idx_010][0], 3);
        assert_eq!(grid.cells[idx_001][0], 4);
    }
}

#[cfg(test)]
mod grid_math_tests {
    use super::*;

    #[test]
    fn test_grid_size_calculation() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 50.0, 200.0),
        };
        let cell_size = 10.0;

        let extent = bounds.max - bounds.min;
        let grid_size_x = (extent.x / cell_size).ceil() as usize + 1;
        let grid_size_y = (extent.y / cell_size).ceil() as usize + 1;
        let grid_size_z = (extent.z / cell_size).ceil() as usize + 1;

        assert_eq!(grid_size_x, 11, "100/10 = 10, +1 = 11");
        assert_eq!(grid_size_y, 6, "50/10 = 5, +1 = 6");
        assert_eq!(grid_size_z, 21, "200/10 = 20, +1 = 21");
    }

    #[test]
    fn test_grid_size_with_non_divisible_extent() {
        let bounds = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(105.0, 53.0, 207.0),
        };
        let cell_size = 10.0;

        let extent = bounds.max - bounds.min;
        let grid_size_x = (extent.x / cell_size).ceil() as usize + 1;
        let grid_size_y = (extent.y / cell_size).ceil() as usize + 1;
        let grid_size_z = (extent.z / cell_size).ceil() as usize + 1;

        assert_eq!(grid_size_x, 12, "105/10 = 10.5, ceil = 11, +1 = 12");
        assert_eq!(grid_size_y, 7, "53/10 = 5.3, ceil = 6, +1 = 7");
        assert_eq!(grid_size_z, 22, "207/10 = 20.7, ceil = 21, +1 = 22");
    }

    #[test]
    fn test_hierarchical_grid_level_sizes() {
        let finest_size = FINEST_CELL_SIZE; // 16.0
        let levels = 4;

        let mut sizes = Vec::new();
        for level in 0..(levels - 1) {
            let cell_size = finest_size * (1 << (levels - 1 - level)) as f32;
            sizes.push(cell_size);
        }

        assert_eq!(sizes[0], 128.0, "Level 0: 16 * 2^3 = 128");
        assert_eq!(sizes[1], 64.0, "Level 1: 16 * 2^2 = 64");
        assert_eq!(sizes[2], 32.0, "Level 2: 16 * 2^1 = 32");
    }
}
