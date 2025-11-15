use crate::types::{BoxData, TriangleData};
use crate::math::AABB;
use glam::Vec3;

pub const GRID_LEVELS: usize = 4;
pub const FINEST_CELL_SIZE: f32 = 16.0;
pub const MAX_OBJECTS_PER_CELL: usize = 256;

fn calculate_grid_dimensions(bounds: &AABB, cell_size: f32) -> [usize; 3] {
    let extent = bounds.max - bounds.min;
    [
        (extent.x / cell_size).ceil() as usize,
        (extent.y / cell_size).ceil() as usize,
        (extent.z / cell_size).ceil() as usize,
    ]
}

const fn compute_cell_index(x: usize, y: usize, z: usize, grid_size: [usize; 3]) -> usize {
    x + y * grid_size[0] + z * grid_size[0] * grid_size[1]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridMetadata {
    pub bounds_min: [f32; 3],
    pub num_levels: u32,
    pub bounds_max: [f32; 3],
    pub finest_cell_size: f32,
    pub grid_sizes: [[u32; 4]; GRID_LEVELS],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FineCellData {
    pub object_indices: [u32; 256],
    pub count: u32,
    pub _pad: [u32; 3],
}

pub struct CoarseGridLevel {
    pub cell_size: f32,
    pub grid_size: [usize; 3],
    pub counts: Vec<u8>,
}

impl CoarseGridLevel {
    pub fn new(bounds: &AABB, cell_size: f32) -> Self {
        let grid_size = calculate_grid_dimensions(bounds, cell_size);
        let total_cells = grid_size[0] * grid_size[1] * grid_size[2];

        Self {
            cell_size,
            grid_size,
            counts: vec![0; total_cells],
        }
    }

    pub fn cell_index(&self, x: usize, y: usize, z: usize) -> usize {
        compute_cell_index(x, y, z, self.grid_size)
    }

    pub fn increment_cell(&mut self, x: usize, y: usize, z: usize) {
        let idx = self.cell_index(x, y, z);
        if self.counts[idx] < 255 {
            self.counts[idx] += 1;
        }
    }
}

pub struct FineGridLevel {
    pub cell_size: f32,
    pub grid_size: [usize; 3],
    pub cells: Vec<Vec<u32>>,
}

impl FineGridLevel {
    pub fn new(bounds: &AABB, cell_size: f32) -> Self {
        let grid_size = calculate_grid_dimensions(bounds, cell_size);
        let total_cells = grid_size[0] * grid_size[1] * grid_size[2];

        Self {
            cell_size,
            grid_size,
            cells: vec![Vec::new(); total_cells],
        }
    }

    pub fn cell_index(&self, x: usize, y: usize, z: usize) -> usize {
        compute_cell_index(x, y, z, self.grid_size)
    }

    pub fn add_object(&mut self, x: usize, y: usize, z: usize, object_id: u32) {
        let idx = self.cell_index(x, y, z);
        if self.cells[idx].len() < MAX_OBJECTS_PER_CELL {
            self.cells[idx].push(object_id);
        } else {
            eprintln!("WARNING: Cell ({}, {}, {}) exceeded MAX_OBJECTS_PER_CELL ({}), dropping object {}",
                     x, y, z, MAX_OBJECTS_PER_CELL, object_id);
        }
    }
}

pub struct HierarchicalGrid {
    pub bounds: AABB,
    pub coarse_levels: Vec<CoarseGridLevel>,
    pub fine_level: FineGridLevel,
}

impl HierarchicalGrid {
    pub fn build(objects: &[BoxData], triangles: &[TriangleData]) -> Self {
        // Compute bounds from both boxes and triangles
        let mut bounds = if !objects.is_empty() {
            objects[0].bounds()
        } else if !triangles.is_empty() {
            triangles[0].bounds()
        } else {
            AABB {
                min: Vec3::splat(-1.0),
                max: Vec3::splat(1.0),
            }
        };

        for obj in &objects[1..] {
            bounds = bounds.union(&obj.bounds());
        }
        for tri in triangles {
            bounds = bounds.union(&tri.bounds());
        }

        let padding = Vec3::splat(1.0);
        bounds.min -= padding;
        bounds.max += padding;

        println!("Grid bounds: {:?} to {:?}", bounds.min, bounds.max);

        let mut coarse_levels = Vec::new();
        for level in 0..(GRID_LEVELS - 1) {
            let cell_size = FINEST_CELL_SIZE * (1 << (GRID_LEVELS - 1 - level)) as f32;
            coarse_levels.push(CoarseGridLevel::new(&bounds, cell_size));
            println!(
                "Coarse level {}: {}x{}x{} cells (size: {})",
                level,
                coarse_levels[level].grid_size[0],
                coarse_levels[level].grid_size[1],
                coarse_levels[level].grid_size[2],
                cell_size
            );
        }

        let fine_level = FineGridLevel::new(&bounds, FINEST_CELL_SIZE);
        println!(
            "Fine level: {}x{}x{} cells (size: {})",
            fine_level.grid_size[0],
            fine_level.grid_size[1],
            fine_level.grid_size[2],
            FINEST_CELL_SIZE
        );

        let mut grid = Self {
            bounds,
            coarse_levels,
            fine_level,
        };

        // Assign boxes (object IDs 0..num_boxes-1)
        for (obj_id, obj) in objects.iter().enumerate() {
            grid.assign_object(obj, obj_id as u32);
        }

        // Assign triangles (object IDs num_boxes..num_boxes+num_triangles-1)
        let num_boxes = objects.len() as u32;
        for (tri_id, tri) in triangles.iter().enumerate() {
            grid.assign_triangle(tri, num_boxes + tri_id as u32);
        }

        let total_coarse_cells: usize = grid
            .coarse_levels
            .iter()
            .map(|level| level.counts.len())
            .sum();
        let occupied_fine_cells = grid
            .fine_level
            .cells
            .iter()
            .filter(|cell| !cell.is_empty())
            .count();

        let max_objects_in_cell = grid
            .fine_level
            .cells
            .iter()
            .map(|cell| cell.len())
            .max()
            .unwrap_or(0);

        let cells_at_capacity = grid
            .fine_level
            .cells
            .iter()
            .filter(|cell| cell.len() >= MAX_OBJECTS_PER_CELL)
            .count();

        println!("Grid stats:");
        println!("  Coarse cells total: {}", total_coarse_cells);
        println!(
            "  Fine cells occupied: {}/{}",
            occupied_fine_cells,
            grid.fine_level.cells.len()
        );
        println!("  Max objects in a cell: {}", max_objects_in_cell);
        println!("  Cells at capacity: {}", cells_at_capacity);

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

        (min_cell.x..=max_cell.x).flat_map(move |x| {
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

        for level in self.coarse_levels.iter_mut() {
            Self::cells_in_bounds(
                obj_min,
                obj_max,
                bounds_min,
                level.cell_size,
                level.grid_size,
            )
            .for_each(|(x, y, z)| level.increment_cell(x, y, z));
        }

        Self::cells_in_bounds(
            obj_min,
            obj_max,
            bounds_min,
            self.fine_level.cell_size,
            self.fine_level.grid_size,
        )
        .for_each(|(x, y, z)| self.fine_level.add_object(x, y, z, obj_id));
    }

    fn assign_triangle(&mut self, tri: &TriangleData, obj_id: u32) {
        let tri_bounds = tri.bounds();
        let obj_min = tri_bounds.min;
        let obj_max = tri_bounds.max;
        let bounds_min = self.bounds.min;

        for level in self.coarse_levels.iter_mut() {
            Self::cells_in_bounds(
                obj_min,
                obj_max,
                bounds_min,
                level.cell_size,
                level.grid_size,
            )
            .for_each(|(x, y, z)| level.increment_cell(x, y, z));
        }

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

    pub fn to_gpu_buffers(&self) -> (GridMetadata, Vec<u8>, Vec<FineCellData>) {
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
                        0,
                    ];
                });
            sizes[GRID_LEVELS - 1] = [
                self.fine_level.grid_size[0] as u32,
                self.fine_level.grid_size[1] as u32,
                self.fine_level.grid_size[2] as u32,
                0,
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

        let all_counts: Vec<u8> = self
            .coarse_levels
            .iter()
            .flat_map(|level| level.counts.iter().copied())
            .collect();

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
