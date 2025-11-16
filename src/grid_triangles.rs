use crate::math::AABB;
use crate::types::TriangleData;
use bytemuck;

pub const TRIANGLE_GRID_LEVELS: usize = 4;
pub const TRIANGLE_FINEST_CELL_SIZE: f32 = 16.0;
pub const MAX_TRIANGLES_PER_CELL: usize = 256;

/// Hierarchical grid for triangles (similar to box grid)
pub struct TriangleGrid {
    pub bounds: AABB,
    pub coarse_levels: Vec<CoarseGridLevel>,
    pub fine_level: FineGridLevel,
}

pub struct CoarseGridLevel {
    pub cell_size: f32,
    pub grid_size: [usize; 3],
    pub counts: Vec<u8>,
}

pub struct FineGridLevel {
    pub cell_size: f32,
    pub grid_size: [usize; 3],
    pub cells: Vec<Vec<u32>>,
}

impl TriangleGrid {
    pub fn build(triangles: &[TriangleData]) -> Self {
        if triangles.is_empty() {
            let default_bounds = AABB {
                min: glam::Vec3::splat(-1.0),
                max: glam::Vec3::splat(1.0),
            };
            return Self {
                bounds: default_bounds,
                coarse_levels: vec![],
                fine_level: FineGridLevel {
                    cell_size: TRIANGLE_FINEST_CELL_SIZE,
                    grid_size: [1, 1, 1],
                    cells: vec![vec![]],
                },
            };
        }

        // Compute overall bounding box
        let mut bounds = triangles[0].bounds();
        for triangle in triangles.iter().skip(1) {
            bounds = AABB::union(&bounds, &triangle.bounds());
        }

        // Add padding
        let padding = glam::Vec3::splat(1.0);
        bounds.min -= padding;
        bounds.max += padding;

        println!("Triangle grid bounds: {:?} to {:?}", bounds.min, bounds.max);

        // Build grid levels
        let mut coarse_levels = Vec::new();
        for level in 0..(TRIANGLE_GRID_LEVELS - 1) {
            let cell_size = TRIANGLE_FINEST_CELL_SIZE * (1 << (TRIANGLE_GRID_LEVELS - 1 - level)) as f32;
            let grid_size = Self::compute_grid_size(&bounds, cell_size);
            let total_cells = grid_size[0] * grid_size[1] * grid_size[2];
            let mut counts = vec![0u8; total_cells];

            // Count triangles in each cell
            for (_tri_idx, triangle) in triangles.iter().enumerate() {
                let tri_bounds = triangle.bounds();
                Self::mark_cells(&bounds, cell_size, &grid_size, &tri_bounds, &mut counts);
            }

            println!("Coarse level {}: {}x{}x{} cells (size: {})",
                level, grid_size[0], grid_size[1], grid_size[2], cell_size);

            coarse_levels.push(CoarseGridLevel {
                cell_size,
                grid_size,
                counts,
            });
        }

        // Build fine level
        let fine_cell_size = TRIANGLE_FINEST_CELL_SIZE;
        let fine_grid_size = Self::compute_grid_size(&bounds, fine_cell_size);
        let fine_total_cells = fine_grid_size[0] * fine_grid_size[1] * fine_grid_size[2];
        let mut fine_cells = vec![Vec::new(); fine_total_cells];

        for (tri_idx, triangle) in triangles.iter().enumerate() {
            let tri_bounds = triangle.bounds();
            Self::insert_into_cells(
                &bounds,
                fine_cell_size,
                &fine_grid_size,
                &tri_bounds,
                tri_idx as u32,
                &mut fine_cells,
            );
        }

        let occupied = fine_cells.iter().filter(|c| !c.is_empty()).count();
        let max_tris = fine_cells.iter().map(|c| c.len()).max().unwrap_or(0);

        println!("Fine level: {}x{}x{} cells (size: {})",
            fine_grid_size[0], fine_grid_size[1], fine_grid_size[2], fine_cell_size);
        println!("Triangle grid stats:");
        println!("  Fine cells occupied: {}/{}", occupied, fine_total_cells);
        println!("  Max triangles in a cell: {}", max_tris);

        Self {
            bounds,
            coarse_levels,
            fine_level: FineGridLevel {
                cell_size: fine_cell_size,
                grid_size: fine_grid_size,
                cells: fine_cells,
            },
        }
    }

    fn compute_grid_size(bounds: &AABB, cell_size: f32) -> [usize; 3] {
        const MIN_CELL_SIZE: f32 = 0.1;
        const MAX_GRID_DIM: usize = 1024;

        // Clamp cell size to prevent memory exhaustion from tiny values
        let safe_cell_size = cell_size.max(MIN_CELL_SIZE);
        let size = bounds.max - bounds.min;

        [
            ((size.x / safe_cell_size).ceil() as usize).clamp(1, MAX_GRID_DIM),
            ((size.y / safe_cell_size).ceil() as usize).clamp(1, MAX_GRID_DIM),
            ((size.z / safe_cell_size).ceil() as usize).clamp(1, MAX_GRID_DIM),
        ]
    }

    fn mark_cells(
        grid_bounds: &AABB,
        cell_size: f32,
        grid_size: &[usize; 3],
        tri_bounds: &AABB,
        counts: &mut [u8],
    ) {
        let min_cell = Self::world_to_cell(grid_bounds, cell_size, tri_bounds.min);
        let max_cell = Self::world_to_cell(grid_bounds, cell_size, tri_bounds.max);

        for x in min_cell.x.max(0)..=(max_cell.x.min(grid_size[0] as i32 - 1)) {
            for y in min_cell.y.max(0)..=(max_cell.y.min(grid_size[1] as i32 - 1)) {
                for z in min_cell.z.max(0)..=(max_cell.z.min(grid_size[2] as i32 - 1)) {
                    // Loop bounds guarantee x,y,z are non-negative and within grid bounds
                    debug_assert!(x >= 0 && y >= 0 && z >= 0);
                    debug_assert!((x as usize) < grid_size[0] && (y as usize) < grid_size[1] && (z as usize) < grid_size[2]);
                    let cell_idx = Self::cell_to_index([x as usize, y as usize, z as usize], grid_size);
                    if cell_idx < counts.len() {
                        counts[cell_idx] = counts[cell_idx].saturating_add(1);
                    }
                }
            }
        }
    }

    fn insert_into_cells(
        grid_bounds: &AABB,
        cell_size: f32,
        grid_size: &[usize; 3],
        tri_bounds: &AABB,
        tri_idx: u32,
        cells: &mut [Vec<u32>],
    ) {
        let min_cell = Self::world_to_cell(grid_bounds, cell_size, tri_bounds.min);
        let max_cell = Self::world_to_cell(grid_bounds, cell_size, tri_bounds.max);

        for x in min_cell.x.max(0)..=(max_cell.x.min(grid_size[0] as i32 - 1)) {
            for y in min_cell.y.max(0)..=(max_cell.y.min(grid_size[1] as i32 - 1)) {
                for z in min_cell.z.max(0)..=(max_cell.z.min(grid_size[2] as i32 - 1)) {
                    // Loop bounds guarantee x,y,z are non-negative and within grid bounds
                    debug_assert!(x >= 0 && y >= 0 && z >= 0);
                    debug_assert!((x as usize) < grid_size[0] && (y as usize) < grid_size[1] && (z as usize) < grid_size[2]);
                    let cell_idx = Self::cell_to_index([x as usize, y as usize, z as usize], grid_size);
                    if cell_idx < cells.len() && cells[cell_idx].len() < MAX_TRIANGLES_PER_CELL {
                        cells[cell_idx].push(tri_idx);
                    }
                }
            }
        }
    }

    fn world_to_cell(grid_bounds: &AABB, cell_size: f32, pos: glam::Vec3) -> glam::IVec3 {
        let relative = pos - grid_bounds.min;
        glam::IVec3::new(
            (relative.x / cell_size).floor() as i32,
            (relative.y / cell_size).floor() as i32,
            (relative.z / cell_size).floor() as i32,
        )
    }

    fn cell_to_index(cell: [usize; 3], grid_size: &[usize; 3]) -> usize {
        cell[0] + cell[1] * grid_size[0] + cell[2] * grid_size[0] * grid_size[1]
    }

    pub fn to_gpu_buffers(&self) -> (TriangleGridMetadata, Vec<u8>, Vec<u8>) {
        let mut grid_sizes = [[0u32; 4]; TRIANGLE_GRID_LEVELS];
        for (i, level) in self.coarse_levels.iter().enumerate() {
            grid_sizes[i] = [
                level.grid_size[0] as u32,
                level.grid_size[1] as u32,
                level.grid_size[2] as u32,
                0,
            ];
        }
        grid_sizes[TRIANGLE_GRID_LEVELS - 1] = [
            self.fine_level.grid_size[0] as u32,
            self.fine_level.grid_size[1] as u32,
            self.fine_level.grid_size[2] as u32,
            0,
        ];

        let metadata = TriangleGridMetadata {
            bounds_min: self.bounds.min.to_array(),
            num_levels: TRIANGLE_GRID_LEVELS as u32,
            bounds_max: self.bounds.max.to_array(),
            finest_cell_size: TRIANGLE_FINEST_CELL_SIZE,
            grid_sizes,
        };

        // Pack coarse level counts
        let mut coarse_data = Vec::new();
        for level in &self.coarse_levels {
            coarse_data.extend_from_slice(&level.counts);
        }

        // Pack fine level cells
        let mut fine_data = Vec::new();
        for cell in &self.fine_level.cells {
            let mut cell_data = FineCellData {
                object_indices: [0u32; MAX_TRIANGLES_PER_CELL],
                count: cell.len().min(MAX_TRIANGLES_PER_CELL) as u32,
                _pad: [0u32; 3],
            };
            for (i, &tri_idx) in cell.iter().take(MAX_TRIANGLES_PER_CELL).enumerate() {
                cell_data.object_indices[i] = tri_idx;
            }
            fine_data.extend_from_slice(bytemuck::bytes_of(&cell_data));
        }

        (metadata, coarse_data, fine_data)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TriangleGridMetadata {
    pub bounds_min: [f32; 3],
    pub num_levels: u32,
    pub bounds_max: [f32; 3],
    pub finest_cell_size: f32,
    pub grid_sizes: [[u32; 4]; TRIANGLE_GRID_LEVELS],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FineCellData {
    pub object_indices: [u32; MAX_TRIANGLES_PER_CELL],
    pub count: u32,
    pub _pad: [u32; 3],
}
