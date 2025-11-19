# Dynamic Worlds & Streaming - The Real Problem

## Problem: Why Can't We Just Load What's Visible?

### Rasterization (Traditional Games) - Easy

```
Camera looking right:

┌──────────────────────────────────────────────────┐
│  ████████████  ║                                 │
│  █ Buildings █  ║    Far away                    │
│  █ (visible) █  ║    (not loaded)                │
│  █           █  ║                                 │
│  ████████████  ║                                 │
│                ║                                 │
│   Camera →     ║                                 │
│   Frustum      ║                                 │
└────────────────║─────────────────────────────────┘
                 │
              Culling plane

Rasterization: Only render what camera SEES
✓ Load only visible chunks
✓ Easy frustum culling
```

### Ray Tracing - HARD!

```
Camera looking right, but with reflections:

┌──────────────────────────────────────────────────┐
│  ████████████  ║  ████████                       │
│  █ Mirror!!! █══╬═►█ Behind █                    │
│  █ (visible) █  ║  █  you!  █                    │
│  █           █  ║  █        █ ← NEEDED FOR       │
│  ████████████  ║  ████████     REFLECTION!       │
│                ║                                 │
│   Camera →     ║                                 │
└────────────────║─────────────────────────────────┘

Problem: Rays bounce ANYWHERE!
✗ Can't predict what's needed
✗ Might need geometry behind camera
✗ Might need geometry far away (for reflections)

Example scenarios needing "invisible" geometry:
1. Reflective surfaces (mirrors, water, metal)
2. Refractive materials (glass, diamonds)
3. Secondary bounces (indirect lighting)
4. Shadows from off-screen objects
```

### The Ray Tracing Dilemma

```
Static Scene (10M triangles):
┌─────────────────────────────────┐
│ Load EVERYTHING: 764 MB         │  ← Current approach
│ Fast BVH traversal              │  ✓ Simple
│ No streaming complexity         │  ✓ Predictable
└─────────────────────────────────┘  ✗ Limited by RAM


Dynamic/Infinite World (∞ triangles):
┌─────────────────────────────────┐
│ Can't load everything! ∞ MB     │
│ Need smarter approach           │  ✗ Complex
│ Stream geometry on-demand       │  ✗ Unpredictable
└─────────────────────────────────┘  ✓ Unlimited scale
```

## Solution 1: Spatial Chunking (Minecraft-style)

### World Division

```
Infinite world divided into chunks:

     Chunk Grid (Top View):

  ┌────┬────┬────┬────┬────┬────┐
  │-2,2│-1,2│0,2 │1,2 │2,2 │3,2 │  Unloaded
  ├────┼────┼────┼────┼────┼────┤
  │-2,1│-1,1│0,1 │1,1 │2,1 │3,1 │
  ├────┼────┼────┼────┼────┼────┤
  │-2,0│-1,0│ 0,0│1,0 │2,0 │3,0 │
  ├────┼────┼────┼────┼────┼────┤  Player
  │-2,-1-1,-1 0,-1 1,-1 2,-1│3,-1│    ▼
  ├────┼────┼────┼────┼────┼────┤
  │-2,-2-1,-2 0,-2 1,-2 2,-2│3,-2│  Loaded
  ├────┼────┼────┼────┼────┼────┤
  │-2,-3-1,-3 0,-3 1,-3 2,-3│3,-3│  Unloaded
  └────┴────┴────┴────┴────┴────┘

Each chunk: 16×16×256 blocks = ~65K triangles
Load radius: 8 chunks = ~4.2M triangles in memory
```

### Chunk BVH System

```rust
struct World {
    chunks: HashMap<(i32, i32), Chunk>,
    active_chunks: Vec<(i32, i32)>,
}

struct Chunk {
    position: (i32, i32),
    bvh: BVHNode,              // Local BVH
    bounds: AABB,              // World-space bounds
    triangles: Vec<Triangle>,
    state: ChunkState,
}

enum ChunkState {
    Unloaded,
    Loading,      // Async generation
    Active,       // In memory + BVH
    Cached,       // Serialized to disk
}
```

### Two-Level BVH

```
Global structure:

Level 1: Chunk BVH (spatial grid)
┌─────────────────────────────────┐
│  World AABB (∞)                 │
│                                 │
│  ┌─────┐  ┌─────┐  ┌─────┐    │
│  │Chunk│  │Chunk│  │Chunk│    │
│  │(-1,0)  │(0,0)│  │(1,0)│    │
│  └─────┘  └─────┘  └─────┘    │
│                                 │
│  ┌─────┐  ┌─────┐  ┌─────┐    │
│  │Chunk│  │Chunk│  │Chunk│    │
│  │(-1,-1 │(0,-1)│(1,-1)│    │
│  └─────┘  └─────┘  └─────┘    │
└─────────────────────────────────┘

Level 2: Per-Chunk BVH
┌─────────────────────────────────┐
│  Chunk (0,0)                    │
│                                 │
│      Root [65K triangles]       │
│         /          \            │
│    Left(32K)    Right(32K)      │
│      /  \          /  \         │
│    ...  ...      ...  ...       │
│    Leaves (~100 tris each)      │
└─────────────────────────────────┘
```

### Ray Traversal with Chunking

```rust
fn trace_chunked_world(world: &World, ray: Ray) -> Option<Hit> {
    let mut closest_hit = None;
    let mut closest_t = f32::INFINITY;

    // 1. Find chunks ray passes through (DDA/voxel traversal)
    let chunks = find_intersecting_chunks(world, ray);

    // 2. Test each chunk's BVH
    for chunk_id in chunks {
        if let Some(chunk) = world.chunks.get(&chunk_id) {
            // Test chunk AABB first
            if !ray.intersects_aabb(&chunk.bounds) {
                continue;  // Skip whole chunk!
            }

            // Traverse chunk's local BVH
            if let Some(hit) = traverse_bvh(&chunk.bvh, ray) {
                if hit.t < closest_t {
                    closest_t = hit.t;
                    closest_hit = Some(hit);
                }
            }
        } else {
            // Chunk not loaded - generate on-demand!
            world.load_chunk(chunk_id);
        }
    }

    closest_hit
}
```

### Chunk Loading Strategy

```
Camera at (0, 0):

Loading Priority (distance-based):

Priority 1 (immediate): Within 2 chunks
  ┌────┬────┬────┐
  │ -1,1│0,1│1,1 │
  ├────┼────┼────┤
  │ -1,0│ ▼ │1,0 │  ← Camera
  ├────┼────┼────┤
  │-1,-1│0,-1│1,-1│
  └────┴────┴────┘

Priority 2 (soon): 3-5 chunks
  Ring around priority 1

Priority 3 (cache): 6-8 chunks
  Keep in memory but low priority

Priority 4 (unload): > 8 chunks
  Serialize to disk, free memory
```

## Solution 2: Octree (Hierarchical Spatial Hash)

### Infinite World with Octree

```
Sparse Octree - Only allocated nodes exist:

                    Root (universe)
                         │
        ┌────────────────┼────────────────┐
        │                │                │
    NW Octant        NE Octant       SW Octant
    (empty)          (player)        (empty)
                         │
        ┌────────────────┼────────────────┐
        │                │                │
    Subdivide        Subdivide        Subdivide
        │                │                │
      Chunk            Chunk            Chunk
    (65K tri)        (65K tri)        (65K tri)

Only allocated nodes consume memory!
Empty = 8 bytes (pointer)
Full chunk = 8 bytes + BVH + triangles
```

### Octree Node Structure

```rust
struct OctreeNode {
    bounds: AABB,
    children: Option<Box<[OctreeNode; 8]>>,  // Lazy allocation
    chunk_data: Option<ChunkData>,
}

// Morton code for spatial hashing
fn morton_encode(x: i32, y: i32, z: i32) -> u64 {
    // Interleave bits for cache-friendly ordering
    // (0,0,0) → 0b000
    // (1,0,0) → 0b001
    // (0,1,0) → 0b010
    // ...
}

// Find node in O(log depth)
fn find_chunk(root: &OctreeNode, pos: Vec3) -> Option<&ChunkData> {
    let mut node = root;

    while let Some(children) = &node.children {
        let idx = which_octant(node.bounds, pos);
        node = &children[idx];
    }

    node.chunk_data.as_ref()
}
```

## Solution 3: Streaming BVH (Most Advanced)

### Concept: Update BVH Incrementally

```
Frame N:                          Frame N+1:
┌──────────────┐                  ┌──────────────┐
│ Loaded: ABCD │                  │ Loaded: BCDE │
│              │    Move →        │              │
│   ████       │                  │     ████     │
│   ▲          │                  │       ▲      │
│              │                  │              │
└──────────────┘                  └──────────────┘

Changes:
- Unload chunk A (serialize to disk)
- Keep chunks B, C, D (already in memory)
- Load chunk E (generate + build BVH)
- Update global BVH (refit, not rebuild!)
```

### Incremental BVH Update

```rust
struct StreamingBVH {
    // Persistent structure
    nodes: Vec<BVHNode>,

    // Dirty flags
    modified_nodes: HashSet<usize>,

    // Chunk mapping
    chunk_to_nodes: HashMap<ChunkId, Vec<usize>>,
}

impl StreamingBVH {
    // O(log n) update instead of O(n log n) rebuild
    fn update_chunk(&mut self, chunk_id: ChunkId, new_triangles: Vec<Triangle>) {
        // 1. Find affected nodes
        let node_indices = self.chunk_to_nodes.get(&chunk_id);

        // 2. Refit AABBs (bottom-up)
        for &node_idx in node_indices {
            self.refit_node(node_idx);  // O(1) per node
        }

        // 3. Propagate changes up tree
        self.propagate_refit(node_idx);  // O(log n)

        // Total: O(log n) vs O(n log n) rebuild!
    }

    fn refit_node(&mut self, idx: usize) {
        match &mut self.nodes[idx] {
            BVHNode::Leaf { bounds, primitives } => {
                // Recompute bounds from primitives
                *bounds = compute_aabb(primitives);
            }
            BVHNode::Internal { bounds, left, right } => {
                // Union of children bounds
                *bounds = left.bounds().union(right.bounds());
            }
        }
    }
}
```

## Solution 4: Hybrid Approach (Practical)

### Combine Multiple Strategies

```
┌─────────────────────────────────────────────────┐
│  Level 1: Frustum + Distance Culling            │
│  ┌───────────────────────────────────────────┐  │
│  │  Only load chunks within:                 │  │
│  │  - View frustum (120° FOV)                │  │
│  │  - Max distance (500m)                    │  │
│  │  - Reflection distance (100m)             │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  Level 2: LOD System                            │
│  ┌───────────────────────────────────────────┐  │
│  │  Close (< 50m):  Full detail (100K tris)  │  │
│  │  Medium (50-200m): Half detail (25K tris) │  │
│  │  Far (200-500m): Low detail (5K tris)     │  │
│  │  Very far (>500m): Impostors (100 tris)   │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  Level 3: Async Streaming                      │
│  ┌───────────────────────────────────────────┐  │
│  │  Background thread:                        │  │
│  │  1. Predict camera movement                │  │
│  │  2. Pre-generate chunks ahead              │  │
│  │  3. Build BVH asynchronously               │  │
│  │  4. Swap in when ready                     │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

### Example: Minecraft-style RT

```rust
struct DynamicWorld {
    // Spatial partitioning
    chunk_size: f32,  // 16 blocks
    chunks: HashMap<ChunkId, Chunk>,

    // Active set (in memory)
    active_radius: f32,  // 8 chunks = 128 blocks

    // Streaming
    load_queue: VecDeque<ChunkId>,
    unload_queue: VecDeque<ChunkId>,

    // Per-frame limits
    max_chunks_per_frame: usize,  // 4 chunks = ~260K triangles
}

impl DynamicWorld {
    fn update(&mut self, camera_pos: Vec3, dt: f32) {
        // 1. Find chunks that should be loaded
        let should_load = self.find_chunks_in_radius(camera_pos);

        // 2. Unload far chunks
        self.chunks.retain(|id, chunk| {
            if !should_load.contains(id) {
                // Serialize to disk
                chunk.save_to_disk();
                false  // Remove from memory
            } else {
                true
            }
        });

        // 3. Load new chunks (rate-limited!)
        let mut loaded_this_frame = 0;
        for chunk_id in should_load {
            if !self.chunks.contains_key(&chunk_id) {
                if loaded_this_frame >= self.max_chunks_per_frame {
                    self.load_queue.push_back(chunk_id);
                    continue;
                }

                // Generate procedurally or load from disk
                let chunk = self.generate_chunk(chunk_id);
                self.chunks.insert(chunk_id, chunk);
                loaded_this_frame += 1;
            }
        }

        // 4. Update BVH for modified chunks
        for (id, chunk) in &mut self.chunks {
            if chunk.is_dirty() {
                chunk.rebuild_local_bvh();  // Fast: only 65K triangles
                chunk.clear_dirty();
            }
        }
    }

    fn find_chunks_in_radius(&self, center: Vec3) -> HashSet<ChunkId> {
        let mut result = HashSet::new();
        let r = (self.active_radius / self.chunk_size) as i32;

        let center_chunk = self.world_to_chunk(center);

        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    let id = (
                        center_chunk.0 + dx,
                        center_chunk.1 + dy,
                        center_chunk.2 + dz,
                    );
                    result.insert(id);
                }
            }
        }

        result
    }
}
```

## Memory Budget Example

### Static Scene (Current)

```
10M triangles:
- Triangle data: 762 MB
- BVH: 1.22 MB
- Total: 764 MB

Single allocation, all or nothing
```

### Dynamic World (Chunked)

```
Budget: 1 GB RAM for geometry

Chunk size: 16×16×256 = 65,536 blocks
Triangle density: 2 triangles/block = 131K triangles/chunk
Chunk memory: 131K × 68 bytes = 8.9 MB

Chunks in budget: 1024 MB / 8.9 MB = 115 chunks
Coverage: 115 chunks × 16 blocks = 1,840 blocks radius
          = 920 meters at 1 block = 0.5m scale

With LOD:
- 30 chunks full detail (close)    = 267 MB
- 50 chunks half detail (medium)   = 222 MB
- 100 chunks low detail (far)      = 178 MB
- 200 chunks impostor (very far)   = 27 MB
Total: 694 MB - MORE coverage with SAME memory!
```

## Comparison Table

| Strategy | Memory | Scalability | Complexity | Best For |
|----------|--------|-------------|------------|----------|
| **Load All** | Fixed | Limited | Low | Static scenes < 1GB |
| **Chunking** | Variable | High | Medium | Minecraft-style |
| **Octree** | Sparse | Very High | Medium | Sparse worlds |
| **Streaming BVH** | Constant | Infinite | High | Open worlds |
| **Hybrid** | Tunable | Infinite | Very High | AAA games |

## Why Current Benchmark Loads Everything

```
Purpose: Stress test BVH performance
- Known scene size (10M triangles)
- Reproducible results
- No streaming complexity
- Pure algorithmic benchmark

Real games would use chunking + streaming!
```

## Next Steps for Dynamic Worlds

1. **Implement chunk system** (16×16×256 grid)
2. **Per-chunk BVH** (65K triangles each)
3. **Async loading** (background thread)
4. **Frustum culling** (don't load behind camera)
5. **LOD system** (distance-based detail)
6. **Prediction** (pre-load in movement direction)

The key: **Don't test what you don't need, don't load what you won't test!**
