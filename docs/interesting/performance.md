# Performance Analysis

Performance benchmarks and optimization insights for ray tracing acceleration structures.

## BVH Construction Performance

The choice of BVH construction algorithm significantly impacts both build time and traversal performance.

```mermaid
graph LR
    A[Primitives] --> B{Construction Algorithm}
    B -->|SAH Binned| C[Quality BVH<br/>~1-2ms/10k tris]
    B -->|SAH Full| D[Optimal BVH<br/>~10-20ms/10k tris]
    B -->|SBVH| E[Best Quality<br/>~50-100ms/10k tris]
    B -->|Morton Code| F[Fast Build<br/>~0.1-0.5ms/10k tris]

    C --> G[Real-time use]
    D --> H[Offline/Precompute]
    E --> I[Static scenes]
    F --> J[Dynamic scenes]

    style C fill:#90EE90
    style D fill:#FFD700
    style E fill:#FF6B6B
    style F fill:#4FC3F7
```

## Traversal Performance Comparison

Different acceleration structures have distinct performance characteristics.

```mermaid
%%{init: {'theme':'dark'}}%%
graph TB
    subgraph "Acceleration Structure Performance"
        A[Ray Query] --> B{Structure Type}

        B -->|BVH| C["O(log n)<br/>Best average"]
        B -->|Grid| D["O(1) to O(n)<br/>Scene dependent"]
        B -->|Octree| E["O(log n)<br/>Memory intensive"]
        B -->|kD-Tree| F["O(log n)<br/>Build complexity"]

        C --> G["✓ Optimal for most scenes<br/>✓ Cache friendly<br/>✓ Easy to update"]
        D --> H["✓ Fast uniform density<br/>✗ Poor sparse scenes<br/>✗ Memory overhead"]
        E --> I["✓ Good for voxels<br/>✗ Deep hierarchies<br/>✗ Unbalanced"]
        F --> J["✓ Tight bounds<br/>✗ Hard to build<br/>✗ Rotation sensitive"]
    end

    style C fill:#4a90e2,stroke:#2d5f9e,color:#fff
    style G fill:#2d5f9e,color:#fff
```

## SAH Split Evaluation Flow

Surface Area Heuristic determines optimal split positions during BVH construction.

```mermaid
flowchart TD
    Start([Start: Node with N primitives]) --> Check{N ≤ leaf_size?}

    Check -->|Yes| Leaf[Create Leaf Node]
    Check -->|No| Bin[Bin primitives across axes]

    Bin --> Eval[Evaluate all split candidates]

    Eval --> SAH["Calculate SAH cost:<br/>C = C_trav + SA_left/SA_parent × N_left × C_int<br/>+ SA_right/SA_parent × N_right × C_int"]

    SAH --> Best{Best cost < leaf cost?}

    Best -->|No| Leaf
    Best -->|Yes| Split[Split at best position]

    Split --> Partition[Partition primitives]

    Partition --> Left[Recurse Left]
    Partition --> Right[Recurse Right]

    Left --> Merge[Create Interior Node]
    Right --> Merge

    Merge --> End([Complete])
    Leaf --> End

    style Start fill:#90EE90
    style SAH fill:#FFD700
    style Best fill:#FF6B6B
    style End fill:#4FC3F7
```

## Memory Layout Impact

Data structure layout has massive impact on cache performance.

```mermaid
graph LR
    subgraph "Array of Structs (AoS)"
        AoS1[Node 0<br/>min max left right]
        AoS2[Node 1<br/>min max left right]
        AoS3[Node 2<br/>min max left right]
    end

    subgraph "Structure of Arrays (SoA)"
        SoA1[All mins: AABB...]
        SoA2[All maxs: AABB...]
        SoA3[All lefts: indices...]
        SoA4[All rights: indices...]
    end

    Ray1[Ray Traversal] -.->|Cache miss| AoS1
    AoS1 -.->|Load 32 bytes| AoS1
    AoS1 -.->|Use 24 bytes| AoS1
    AoS1 -.->|Waste 8 bytes| AoS1

    Ray2[Ray Traversal] -.->|SIMD 4-wide| SoA1
    SoA1 -.->|Test 4 AABBs| SoA1
    SoA1 -.->|100% utilization| SoA1

    style SoA1 fill:#90EE90
    style Ray2 fill:#4FC3F7
```

## Render Pipeline Integration

How BVH fits into the complete ray tracing pipeline.

```mermaid
sequenceDiagram
    participant App as Application
    participant BVH as BVH Builder
    participant GPU as GPU/Shader
    participant Mem as Memory

    App->>BVH: Submit geometry
    BVH->>BVH: Build acceleration structure
    Note over BVH: SAH partitioning<br/>~1-2ms per 10k tris

    BVH->>Mem: Upload BVH to GPU
    Note over Mem: Compact layout<br/>32 bytes/node

    App->>GPU: Dispatch rays

    loop Per Ray
        GPU->>Mem: Fetch BVH node
        GPU->>GPU: Test AABB intersection

        alt Hit Interior Node
            GPU->>GPU: Push children to stack
        else Hit Leaf Node
            GPU->>GPU: Test primitives
        else Miss
            GPU->>GPU: Pop stack
        end
    end

    GPU->>App: Return intersections
```

## Performance Metrics

### Typical Numbers (RTX 4090, 1M triangles)

| Metric | Value | Notes |
|--------|-------|-------|
| BVH Build | 10-15ms | SAH binned, 16 bins |
| BVH Nodes | ~2M nodes | 2N-1 for N primitives |
| Memory | ~64 MB | 32 bytes/node |
| Traversal | 15-25 steps | Average ray depth |
| Throughput | 2-4 Grays/s | Scene dependent |

### Optimization Impact

```mermaid
%%{init: {'theme':'dark'}}%%
pie title "Ray Tracing Performance Breakdown"
    "BVH Traversal" : 45
    "Primitive Intersection" : 30
    "Shading" : 15
    "Memory Bandwidth" : 10
```

## Cache Behavior

Understanding cache patterns is critical for performance.

```mermaid
graph TD
    A[Ray at Root] --> B{Cache Status}

    B -->|Cold| C[L3 Cache Miss<br/>~100 cycles]
    B -->|Warm| D[L1 Cache Hit<br/>~4 cycles]

    C --> E[Fetch from DRAM]
    E --> F[Load 64-byte line]
    F --> G[Contains 2 nodes]

    D --> H[Process immediately]
    G --> H

    H --> I{Coherent Access?}

    I -->|Yes| J["Prefetch next level<br/>Cache hit rate: ~95%"]
    I -->|No| K["Random access<br/>Cache hit rate: ~60%"]

    style D fill:#90EE90
    style C fill:#FF6B6B
    style J fill:#4FC3F7
```

## Real-World Optimization Example

```mermaid
timeline
    title BVH Optimization Journey
    section Naive
        Basic BVH : Median split
                  : 120 fps
                  : Poor quality
    section SAH
        SAH Binned : 8 bins
                   : 95 fps
                   : Better quality
        SAH Binned : 16 bins
                   : 85 fps
                   : Good quality
    section SIMD
        4-wide AABB : SOA layout
                    : 110 fps
                    : Same quality
    section Memory
        Compressed : Quantized bounds
                   : 130 fps
                   : Slight quality loss
    section Final
        All optimizations : SIMD + Compressed + SAH
                          : 144 fps
                          : Target achieved!
```

## Recommendations

```mermaid
graph TD
    Start{Use Case} --> Dynamic{Dynamic Scene?}
    Start --> Static{Static Scene?}

    Dynamic -->|Yes| Fast[Fast Build<br/>Morton/Linear]
    Fast --> Refit{Refit viable?}
    Refit -->|Yes| RefitBVH[Refit existing BVH<br/>~0.1ms]
    Refit -->|No| Rebuild[Rebuild each frame<br/>~1ms]

    Static -->|Yes| Quality[Quality Build<br/>SAH/SBVH]
    Quality --> Precompute[Precompute offline<br/>100ms+ acceptable]

    RefitBVH --> TLAS[Use TLAS/BLAS<br/>for instancing]
    Rebuild --> TLAS
    Precompute --> Compress[Compress for GPU<br/>save bandwidth]

    style RefitBVH fill:#90EE90
    style Precompute fill:#4FC3F7
    style TLAS fill:#FFD700
```

## Related Topics

- [AABB](/interesting/aabb) - Core bounding primitive
- [BVH](/interesting/bvh) - Full construction details
- [Bounding Spheres](/interesting/bounding-spheres) - Alternative bounds
