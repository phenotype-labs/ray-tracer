use glam::Vec3;
use crate::types::{AABB, BoxData};

pub const GRID_LEVELS: usize = 4;
pub const FINEST_CELL_SIZE: f32 = 16.0;  // Smallest cells: 16x16x16 units
pub const MAX_OBJECTS_PER_CELL: usize = 64;

/// Grid metadata for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridMetadata {
    pub bounds_min: [f32; 3],
    pub num_levels: u32,
    pub bounds_max: [f32; 3],
    pub finest_cell_size: f32,
    pub grid_sizes: [[u32; 4]; GRID_LEVELS],  // Size of each level (padded to vec4)
}

/// Fine grid cell data for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FineCellData {
    pub object_indices: [u32; MAX_OBJECTS_PER_CELL],
    pub count: u32,
    pub _pad: [u32; 3],
}

/// Coarse grid level (stores only object count)
pub struct CoarseGridLevel {
    pub cell_size: f32,
    pub grid_size: [usize; 3],
    pub counts: Vec<u8>,  // Flattened 3D array of object counts
}

impl CoarseGridLevel {
    pub fn new(bounds: &AABB, cell_size: f32) -> Self {
        let extent = bounds.max - bounds.min;
        let grid_size = [
            (extent.x / cell_size).ceil() as usize + 1,
            (extent.y / cell_size).ceil() as usize + 1,
            (extent.z / cell_size).ceil() as usize + 1,
        ];

        let total_cells = grid_size[0] * grid_size[1] * grid_size[2];

        Self {
            cell_size,
            grid_size,
            counts: vec![0; total_cells],
        }
    }

    pub fn cell_index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.grid_size[0] + z * self.grid_size[0] * self.grid_size[1]
    }

    pub fn increment_cell(&mut self, x: usize, y: usize, z: usize) {
        let idx = self.cell_index(x, y, z);
        if self.counts[idx] < 255 {
            self.counts[idx] += 1;
        }
    }
}

/// Finest grid level (stores actual object indices)
pub struct FineGridLevel {
    pub cell_size: f32,
    pub grid_size: [usize; 3],
    pub cells: Vec<Vec<u32>>,  // Each cell contains object indices
}

impl FineGridLevel {
    pub fn new(bounds: &AABB, cell_size: f32) -> Self {
        let extent = bounds.max - bounds.min;
        let grid_size = [
            (extent.x / cell_size).ceil() as usize + 1,
            (extent.y / cell_size).ceil() as usize + 1,
            (extent.z / cell_size).ceil() as usize + 1,
        ];

        let total_cells = grid_size[0] * grid_size[1] * grid_size[2];

        Self {
            cell_size,
            grid_size,
            cells: vec![Vec::new(); total_cells],
        }
    }

    pub fn cell_index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.grid_size[0] + z * self.grid_size[0] * self.grid_size[1]
    }

    pub fn add_object(&mut self, x: usize, y: usize, z: usize, object_id: u32) {
        let idx = self.cell_index(x, y, z);
        if self.cells[idx].len() < MAX_OBJECTS_PER_CELL {
            self.cells[idx].push(object_id);
        }
    }
}

/// Complete hierarchical grid structure
pub struct HierarchicalGrid {
    pub bounds: AABB,
    pub coarse_levels: Vec<CoarseGridLevel>,
    pub fine_level: FineGridLevel,
}

impl HierarchicalGrid {
    pub fn build(objects: &[BoxData]) -> Self {
        // Calculate scene bounds
        let mut bounds = objects[0].bounds();
        for obj in &objects[1..] {
            bounds = bounds.union(&obj.bounds());
        }

        // Add padding
        let padding = Vec3::splat(1.0);
        bounds.min -= padding;
        bounds.max += padding;

        println!("Grid bounds: {:?} to {:?}", bounds.min, bounds.max);

        // Create levels (from coarse to fine)
        let mut coarse_levels = Vec::new();
        for level in 0..(GRID_LEVELS - 1) {
            let cell_size = FINEST_CELL_SIZE * (1 << (GRID_LEVELS - 1 - level)) as f32;
            coarse_levels.push(CoarseGridLevel::new(&bounds, cell_size));
            println!("Coarse level {}: {}x{}x{} cells (size: {})",
                level,
                coarse_levels[level].grid_size[0],
                coarse_levels[level].grid_size[1],
                coarse_levels[level].grid_size[2],
                cell_size);
        }

        let fine_level = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);
        println!("Fine level: {}x{}x{} cells (size: {})",
            fine_level.grid_size[0],
            fine_level.grid_size[1],
            fine_level.grid_size[2],
            FINEST_CELL_SIZE);

        let mut grid = Self {
            bounds,
            coarse_levels,
            fine_level,
        };

        // Assign objects to grid
        for (obj_id, obj) in objects.iter().enumerate() {
            grid.assign_object(obj, obj_id as u32);
        }

        // Print statistics
        let total_coarse_cells: usize = grid.coarse_levels.iter()
            .map(|level| level.counts.len())
            .sum();
        let occupied_fine_cells = grid.fine_level.cells.iter()
            .filter(|cell| !cell.is_empty())
            .count();

        println!("Grid stats:");
        println!("  Coarse cells total: {}", total_coarse_cells);
        println!("  Fine cells occupied: {}/{}",
            occupied_fine_cells,
            grid.fine_level.cells.len());

        grid
    }

    fn cells_in_bounds(
        obj_min: Vec3,
        obj_max: Vec3,
        bounds_min: Vec3,
        cell_size: f32,
        grid_size: [usize; 3],
    ) -> impl Iterator<Item = (usize, usize, usize)> {
        let min_cell = Self::world_to_cell_static(&obj_min, bounds_min, cell_size);
        let max_cell = Self::world_to_cell_static(&obj_max, bounds_min, cell_size);

        (min_cell.x..=max_cell.x)
            .flat_map(move |x| {
                (min_cell.y..=max_cell.y).flat_map(move |y| {
                    (min_cell.z..=max_cell.z).filter_map(move |z| {
                        let (xu, yu, zu) = (x as usize, y as usize, z as usize);
                        (xu < grid_size[0] && yu < grid_size[1] && zu < grid_size[2])
                            .then_some((xu, yu, zu))
                    })
                })
            })
    }

    fn assign_object(&mut self, obj: &BoxData, obj_id: u32) {
        let obj_min = Vec3::from_array(obj.min);
        let obj_max = Vec3::from_array(obj.max);
        let bounds_min = self.bounds.min;

        // Assign to all coarse levels
        for level in self.coarse_levels.iter_mut() {
            Self::cells_in_bounds(obj_min, obj_max, bounds_min, level.cell_size, level.grid_size)
                .for_each(|(x, y, z)| level.increment_cell(x, y, z));
        }

        // Assign to fine level
        Self::cells_in_bounds(
            obj_min,
            obj_max,
            bounds_min,
            self.fine_level.cell_size,
            self.fine_level.grid_size,
        )
        .for_each(|(x, y, z)| self.fine_level.add_object(x, y, z, obj_id));
    }

    fn world_to_cell_static(pos: &Vec3, bounds_min: Vec3, cell_size: f32) -> glam::UVec3 {
        let rel_pos = *pos - bounds_min;
        glam::UVec3::new(
            (rel_pos.x / cell_size).floor().max(0.0) as u32,
            (rel_pos.y / cell_size).floor().max(0.0) as u32,
            (rel_pos.z / cell_size).floor().max(0.0) as u32,
        )
    }

    /// Flatten grid to GPU-compatible buffers
    pub fn to_gpu_buffers(&self) -> (GridMetadata, Vec<u8>, Vec<FineCellData>) {
        // Create metadata (pad vec3 to vec4 for WGSL alignment)
        let grid_sizes: [[u32; 4]; GRID_LEVELS] = {
            let mut sizes = [[0u32; 4]; GRID_LEVELS];
            self.coarse_levels
                .iter()
                .enumerate()
                .for_each(|(i, level)| {
                    sizes[i] = [
                        level.grid_size[0] as u32,
                        level.grid_size[1] as u32,
                        level.grid_size[2] as u32,
                        0, // Padding
                    ];
                });
            sizes[GRID_LEVELS - 1] = [
                self.fine_level.grid_size[0] as u32,
                self.fine_level.grid_size[1] as u32,
                self.fine_level.grid_size[2] as u32,
                0, // Padding
            ];
            sizes
        };

        let metadata = GridMetadata {
            bounds_min: self.bounds.min.to_array(),
            num_levels: GRID_LEVELS as u32,
            bounds_max: self.bounds.max.to_array(),
            finest_cell_size: FINEST_CELL_SIZE,
            grid_sizes,
        };

        // Flatten coarse level counts
        let all_counts: Vec<u8> = self
            .coarse_levels
            .iter()
            .flat_map(|level| level.counts.iter().copied())
            .collect();

        // Flatten fine level cells
        let fine_cells: Vec<FineCellData> = self
            .fine_level
            .cells
            .iter()
            .map(|cell| {
                let mut object_indices = [0u32; MAX_OBJECTS_PER_CELL];
                cell.iter()
                    .take(MAX_OBJECTS_PER_CELL)
                    .enumerate()
                    .for_each(|(i, &obj_id)| object_indices[i] = obj_id);

                FineCellData {
                    object_indices,
                    count: cell.len() as u32,
                    _pad: [0; 3],
                }
            })
            .collect();

        (metadata, all_counts, fine_cells)
    }
}
