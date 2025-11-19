use crate::math::AABB;
use glam::Vec3;

/// Maximum primitives per leaf node before splitting
const MAX_LEAF_SIZE: usize = 4;

/// Number of SAH buckets for binned building
const SAH_BUCKETS: usize = 12;

/// BVH node using compact representation
#[derive(Clone, Debug)]
pub enum BVHNode {
    Leaf {
        bounds: AABB,
        primitive_indices: Vec<u32>,
    },
    Internal {
        bounds: AABB,
        left: Box<BVHNode>,
        right: Box<BVHNode>,
    },
}

/// Primitive trait for objects that can be inserted into BVH
pub trait BVHPrimitive {
    fn bounds(&self) -> AABB;
    fn centroid(&self) -> Vec3 {
        self.bounds().center()
    }
}

/// BVH build statistics for profiling
#[derive(Debug, Clone, Copy)]
pub struct BVHStats {
    pub num_nodes: usize,
    pub num_leaves: usize,
    pub max_depth: usize,
    pub total_primitives: usize,
    pub avg_leaf_size: f32,
}

impl BVHNode {
    /// Build BVH using SAH (Surface Area Heuristic) for optimal splits
    pub fn build<P: BVHPrimitive>(primitives: &[P]) -> Self {
        let indices: Vec<u32> = (0..primitives.len() as u32).collect();
        Self::build_recursive(primitives, indices, 0)
    }

    fn build_recursive<P: BVHPrimitive>(
        primitives: &[P],
        mut indices: Vec<u32>,
        depth: usize,
    ) -> Self {
        // Compute bounds for all primitives in this node
        let bounds = indices.iter().fold(
            primitives[indices[0] as usize].bounds(),
            |acc, &idx| acc.union(&primitives[idx as usize].bounds()),
        );

        // Create leaf if we have few primitives
        if indices.len() <= MAX_LEAF_SIZE {
            return BVHNode::Leaf {
                bounds,
                primitive_indices: indices,
            };
        }

        // Find best split using SAH
        let (split_axis, split_pos) = Self::find_best_split(primitives, &indices, &bounds);

        // Partition primitives based on split
        let mid = Self::partition_primitives(primitives, &mut indices, split_axis, split_pos);

        // If partition failed, create leaf
        if mid == 0 || mid == indices.len() {
            return BVHNode::Leaf {
                bounds,
                primitive_indices: indices,
            };
        }

        // Split indices and build children
        let right_indices = indices.split_off(mid);
        let left = Box::new(Self::build_recursive(primitives, indices, depth + 1));
        let right = Box::new(Self::build_recursive(primitives, right_indices, depth + 1));

        BVHNode::Internal {
            bounds,
            left,
            right,
        }
    }

    /// Find best split using binned SAH
    fn find_best_split<P: BVHPrimitive>(
        primitives: &[P],
        indices: &[u32],
        bounds: &AABB,
    ) -> (usize, f32) {
        let mut best_cost = f32::INFINITY;
        let mut best_axis = 0;
        let mut best_pos = 0.0;

        // Try each axis
        for axis in 0..3 {
            let (cost, pos) = Self::evaluate_sah_axis(primitives, indices, bounds, axis);
            if cost < best_cost {
                best_cost = cost;
                best_axis = axis;
                best_pos = pos;
            }
        }

        (best_axis, best_pos)
    }

    /// Evaluate SAH cost for a given axis using binning
    fn evaluate_sah_axis<P: BVHPrimitive>(
        primitives: &[P],
        indices: &[u32],
        bounds: &AABB,
        axis: usize,
    ) -> (f32, f32) {
        // Initialize buckets
        let mut bucket_bounds: Vec<Option<AABB>> = vec![None; SAH_BUCKETS];
        let mut bucket_counts = vec![0; SAH_BUCKETS];

        let extent = bounds.max - bounds.min;
        let axis_extent = extent[axis];

        if axis_extent < 1e-6 {
            return (f32::INFINITY, 0.0);
        }

        // Assign primitives to buckets
        for &idx in indices {
            let centroid = primitives[idx as usize].centroid();
            let offset = (centroid[axis] - bounds.min[axis]) / axis_extent;
            let bucket_idx = ((offset * SAH_BUCKETS as f32) as usize).min(SAH_BUCKETS - 1);

            bucket_counts[bucket_idx] += 1;
            let prim_bounds = primitives[idx as usize].bounds();
            bucket_bounds[bucket_idx] = Some(match bucket_bounds[bucket_idx] {
                Some(b) => b.union(&prim_bounds),
                None => prim_bounds,
            });
        }

        // Sweep to find best split
        let mut best_cost = f32::INFINITY;
        let mut best_split = 0;

        for split in 1..SAH_BUCKETS {
            let (left_bounds, left_count) =
                Self::accumulate_buckets(&bucket_bounds, &bucket_counts, 0, split);
            let (right_bounds, right_count) =
                Self::accumulate_buckets(&bucket_bounds, &bucket_counts, split, SAH_BUCKETS);

            if let (Some(lb), Some(rb)) = (left_bounds, right_bounds) {
                let cost = Self::sah_cost(
                    lb.surface_area(),
                    left_count,
                    rb.surface_area(),
                    right_count,
                );

                if cost < best_cost {
                    best_cost = cost;
                    best_split = split;
                }
            }
        }

        // Calculate split position
        let split_pos = bounds.min[axis] + (best_split as f32 / SAH_BUCKETS as f32) * axis_extent;

        (best_cost, split_pos)
    }

    fn accumulate_buckets(
        bucket_bounds: &[Option<AABB>],
        bucket_counts: &[usize],
        start: usize,
        end: usize,
    ) -> (Option<AABB>, usize) {
        let mut combined_bounds: Option<AABB> = None;
        let mut total_count = 0;

        for i in start..end {
            if let Some(bounds) = bucket_bounds[i] {
                combined_bounds = Some(match combined_bounds {
                    Some(b) => b.union(&bounds),
                    None => bounds,
                });
                total_count += bucket_counts[i];
            }
        }

        (combined_bounds, total_count)
    }

    /// SAH cost function
    fn sah_cost(left_area: f32, left_count: usize, right_area: f32, right_count: usize) -> f32 {
        const TRAVERSAL_COST: f32 = 0.125;
        const INTERSECTION_COST: f32 = 1.0;

        TRAVERSAL_COST
            + INTERSECTION_COST * (left_area * left_count as f32 + right_area * right_count as f32)
    }

    /// Partition primitives along axis at split position
    fn partition_primitives<P: BVHPrimitive>(
        primitives: &[P],
        indices: &mut [u32],
        axis: usize,
        split_pos: f32,
    ) -> usize {
        let mut left = 0;
        let mut right = indices.len();

        while left < right {
            let centroid = primitives[indices[left] as usize].centroid();
            if centroid[axis] < split_pos {
                left += 1;
            } else {
                right -= 1;
                indices.swap(left, right);
            }
        }

        left
    }

    /// Get bounding box for this node
    pub fn bounds(&self) -> &AABB {
        match self {
            BVHNode::Leaf { bounds, .. } => bounds,
            BVHNode::Internal { bounds, .. } => bounds,
        }
    }

    /// Gather statistics about the BVH
    pub fn stats(&self) -> BVHStats {
        let mut stats = BVHStats {
            num_nodes: 0,
            num_leaves: 0,
            max_depth: 0,
            total_primitives: 0,
            avg_leaf_size: 0.0,
        };

        self.gather_stats(&mut stats, 0);

        if stats.num_leaves > 0 {
            stats.avg_leaf_size = stats.total_primitives as f32 / stats.num_leaves as f32;
        }

        stats
    }

    fn gather_stats(&self, stats: &mut BVHStats, depth: usize) {
        stats.num_nodes += 1;
        stats.max_depth = stats.max_depth.max(depth);

        match self {
            BVHNode::Leaf {
                primitive_indices, ..
            } => {
                stats.num_leaves += 1;
                stats.total_primitives += primitive_indices.len();
            }
            BVHNode::Internal { left, right, .. } => {
                left.gather_stats(stats, depth + 1);
                right.gather_stats(stats, depth + 1);
            }
        }
    }

    /// Flatten BVH to GPU-friendly linear array format
    pub fn flatten(&self) -> Vec<FlatBVHNode> {
        let mut nodes = Vec::new();
        self.flatten_recursive(&mut nodes);
        nodes
    }

    fn flatten_recursive(&self, nodes: &mut Vec<FlatBVHNode>) -> u32 {
        let node_idx = nodes.len() as u32;

        match self {
            BVHNode::Leaf {
                bounds,
                primitive_indices,
            } => {
                nodes.push(FlatBVHNode {
                    bounds_min: bounds.min.to_array(),
                    prim_count: primitive_indices.len() as u32,
                    bounds_max: bounds.max.to_array(),
                    prim_offset: primitive_indices[0],
                });
            }
            BVHNode::Internal { bounds, left, right } => {
                // Reserve space for this node
                nodes.push(FlatBVHNode::default());

                // Flatten left child first
                let _left_idx = left.flatten_recursive(nodes);

                // Flatten right child
                let right_idx = right.flatten_recursive(nodes);

                // Update this node with right child offset
                nodes[node_idx as usize] = FlatBVHNode {
                    bounds_min: bounds.min.to_array(),
                    prim_count: 0, // 0 indicates internal node
                    bounds_max: bounds.max.to_array(),
                    prim_offset: right_idx,
                };
            }
        }

        node_idx
    }
}

/// GPU-friendly flat BVH node representation
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FlatBVHNode {
    pub bounds_min: [f32; 3],
    pub prim_count: u32,      // 0 = internal node, >0 = leaf with count
    pub bounds_max: [f32; 3],
    pub prim_offset: u32,     // For leaf: first primitive index, for internal: right child offset
}

impl Default for FlatBVHNode {
    fn default() -> Self {
        Self {
            bounds_min: [0.0; 3],
            prim_count: 0,
            bounds_max: [0.0; 3],
            prim_offset: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestPrimitive {
        bounds: AABB,
    }

    impl BVHPrimitive for TestPrimitive {
        fn bounds(&self) -> AABB {
            self.bounds
        }
    }

    #[test]
    fn test_bvh_single_primitive() {
        let prims = vec![TestPrimitive {
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
        }];

        let bvh = BVHNode::build(&prims);
        match bvh {
            BVHNode::Leaf {
                primitive_indices, ..
            } => {
                assert_eq!(primitive_indices.len(), 1);
                assert_eq!(primitive_indices[0], 0);
            }
            _ => panic!("Expected leaf node"),
        }
    }

    #[test]
    fn test_bvh_split() {
        let prims = vec![
            TestPrimitive {
                bounds: AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
            },
            TestPrimitive {
                bounds: AABB::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(11.0, 1.0, 1.0)),
            },
            TestPrimitive {
                bounds: AABB::new(Vec3::new(20.0, 0.0, 0.0), Vec3::new(21.0, 1.0, 1.0)),
            },
            TestPrimitive {
                bounds: AABB::new(Vec3::new(30.0, 0.0, 0.0), Vec3::new(31.0, 1.0, 1.0)),
            },
            TestPrimitive {
                bounds: AABB::new(Vec3::new(40.0, 0.0, 0.0), Vec3::new(41.0, 1.0, 1.0)),
            },
        ];

        let bvh = BVHNode::build(&prims);

        // Should create internal node since we have more than MAX_LEAF_SIZE primitives
        match bvh {
            BVHNode::Internal { .. } => {
                // Success
            }
            BVHNode::Leaf { .. } => panic!("Expected internal node for 5 primitives"),
        }
    }

    #[test]
    fn test_bvh_stats() {
        let prims: Vec<_> = (0..10)
            .map(|i| TestPrimitive {
                bounds: AABB::new(
                    Vec3::new(i as f32 * 10.0, 0.0, 0.0),
                    Vec3::new(i as f32 * 10.0 + 1.0, 1.0, 1.0),
                ),
            })
            .collect();

        let bvh = BVHNode::build(&prims);
        let stats = bvh.stats();

        assert_eq!(stats.total_primitives, 10);
        assert!(stats.num_leaves > 0);
        assert!(stats.max_depth > 0);
        assert!(stats.avg_leaf_size > 0.0);
    }

    #[test]
    fn test_bvh_flatten() {
        let prims = vec![
            TestPrimitive {
                bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            },
            TestPrimitive {
                bounds: AABB::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(11.0, 1.0, 1.0)),
            },
        ];

        let bvh = BVHNode::build(&prims);
        let flat = bvh.flatten();

        assert!(!flat.is_empty());
        // Root should have valid bounds
        assert!(flat[0].bounds_min[0] <= flat[0].bounds_max[0]);
        assert!(flat[0].bounds_min[1] <= flat[0].bounds_max[1]);
        assert!(flat[0].bounds_min[2] <= flat[0].bounds_max[2]);
    }

    #[test]
    fn test_sah_cost_calculation() {
        let cost = BVHNode::sah_cost(100.0, 5, 200.0, 10);
        assert!(cost > 0.0);

        // Smaller areas and counts should have lower cost
        let smaller_cost = BVHNode::sah_cost(50.0, 2, 50.0, 2);
        assert!(smaller_cost < cost);
    }

    #[test]
    fn test_bounds_union_in_build() {
        let prims = vec![
            TestPrimitive {
                bounds: AABB::new(Vec3::new(-5.0, -5.0, -5.0), Vec3::new(0.0, 0.0, 0.0)),
            },
            TestPrimitive {
                bounds: AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0)),
            },
        ];

        let bvh = BVHNode::build(&prims);
        let bounds = bvh.bounds();

        // Bounds should encompass both primitives
        assert_eq!(bounds.min, Vec3::new(-5.0, -5.0, -5.0));
        assert_eq!(bounds.max, Vec3::new(5.0, 5.0, 5.0));
    }
}
