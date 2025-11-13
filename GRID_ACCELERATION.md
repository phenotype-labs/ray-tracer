# Grid Acceleration Strategy

## The Problem

Without acceleration, each ray must test intersection against **all objects** in the scene:
- 1000 objects × 480,000 rays = 480 million intersection tests per frame
- Very slow!

## Our Solution: Hierarchical Grid with Box Duplication

### Core Strategy

**Pre-compute phase (CPU):** Store each box in ALL grid cells it overlaps
**Ray tracing phase (GPU):** Ray only checks its current cell

This trades memory (duplicating box references) for speed (single cell lookup).

## How It Works

### 1. Building the Grid (grid.rs:146-217)

For each box in the scene:

```rust
// Example box
let box = BoxData {
    min: Vec3::new(9.0, 9.0, 9.0),
    max: Vec3::new(21.0, 21.0, 21.0),
    // This box spans 12 units on each axis
};
```

Calculate which cells it overlaps:

```rust
let cell_size = 10.0;

// Min corner → cell (0, 0, 0)
let min_cell = world_to_cell(Vec3::new(9.0, 9.0, 9.0), cell_size);
// = floor(9/10) = (0, 0, 0)

// Max corner → cell (2, 2, 2)
let max_cell = world_to_cell(Vec3::new(21.0, 21.0, 21.0), cell_size);
// = floor(21/10) = (2, 2, 2)
```

**Assign box ID to ALL cells from (0,0,0) to (2,2,2):**

```rust
for x in 0..=2 {
    for y in 0..=2 {
        for z in 0..=2 {
            fine_level.cells[x][y][z].push(box_id);
        }
    }
}
```

The box ID is now stored in **27 cells** (3×3×3).

### 2. Ray Tracing (raytracer_grid.wgsl:246-258)

When a ray traces through the grid:

```wgsl
// Calculate which cell ray is currently in
var current_cell = world_to_cell(ray.origin, cell_size);
// e.g., current_cell = (1, 1, 1)

// Get objects in THIS cell only (no neighbor checking!)
let fine_idx = get_fine_index(current_cell);
let cell_data = fine_cells[fine_idx];

// Test ray against each object in this cell
for (var j = 0u; j < cell_data.count; j++) {
    let obj_idx = cell_data.object_indices[j];
    let hit = intersect_box(ray, boxes[obj_idx], camera.time);
    // ...
}
```

**Key point:** No neighbor cells checked! The box is already present in this cell's list.

### 3. DDA Ray Marching (raytracer_grid.wgsl:266-275)

Ray steps through cells using Digital Differential Analyzer algorithm:

```wgsl
// Move to next cell
if t_max.x < t_max.y && t_max.x < t_max.z {
    current_cell.x += step.x;  // Step in X direction
} else if t_max.y < t_max.z {
    current_cell.y += step.y;  // Step in Y direction
} else {
    current_cell.z += step.z;  // Step in Z direction
}
```

Each step, the ray queries only the new cell's object list.

## Why This Design?

### Alternative: Store Box Only at Center

```rust
// Store box only where its center is
let center = (box.min + box.max) / 2.0;
let center_cell = world_to_cell(center);
fine_level.cells[center_cell].push(box_id);  // Only one cell
```

**Problem:** Ray must check neighboring cells:

```rust
// Ray in cell (0,0,0) must check all 27 neighbors
for dx in -1..=1 {
    for dy in -1..=1 {
        for dz in -1..=1 {
            let neighbor = current_cell + IVec3::new(dx, dy, dz);
            check_boxes_in_cell(neighbor);
        }
    }
}
```

### Trade-off Comparison

| Approach | Memory | Ray Query Cost |
|----------|--------|----------------|
| **Current (duplicate box refs)** | Box ID stored in ~27 cells | Check 1 cell |
| **Alternative (center only)** | Box ID stored in 1 cell | Check 27 cells |

**On GPU:** 27 array accesses is MUCH slower than storing an extra 26 u32 values (104 bytes).

## Memory Layout

```
Master Box Array (renderer.rs:44):
┌────────────────────────────────┐
│ boxes[0]: {min, max, color}    │ ← Actual geometry
│ boxes[1]: {min, max, color}    │
│ boxes[2]: {min, max, color}    │
│ ...                            │
└────────────────────────────────┘
         ↑ referenced by ID
         │
Fine Grid (grid.rs:62):
┌────────────────────────────────┐
│ cell[0,0,0]: [0, 5, 12]        │ ← Box IDs only (u32)
│ cell[0,0,1]: [0, 3]            │
│ cell[1,1,1]: [0, 5, 7, 12, 15] │ ← Box 0 appears in multiple cells
│ ...                            │
└────────────────────────────────┘
```

Each fine cell stores:
- `object_indices: [u32; 64]` - up to 64 box IDs
- `count: u32` - how many boxes in this cell

## Performance

**Without grid:**
- 1000 boxes × 480,000 rays = 480,000,000 tests

**With grid (assuming 10 boxes per cell average):**
- 480,000 rays × 5 cells traversed × 10 boxes = 24,000,000 tests
- **20× faster!**

## Code References

- **Grid building:** `grid.rs:173-217` (cells_in_bounds, assign_object)
- **Box assignment:** `grid.rs:90-95` (add_object)
- **Ray query:** `raytracer_grid.wgsl:246-258`
- **DDA marching:** `raytracer_grid.wgsl:266-275`

## Key Insight

> Boxes are assigned to **all cells they touch** during pre-computation (CPU),
> so rays only need to check **their current cell** during tracing (GPU).

This is the opposite of what you might expect (store once, query many), but it's optimized for GPU parallel execution where memory is cheap and branching is expensive.
