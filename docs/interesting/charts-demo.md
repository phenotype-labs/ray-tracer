---
title: Interactive Charts Demo
---

# Interactive Performance Charts

This page demonstrates interactive Chart.js visualizations for ray tracing performance metrics.

## BVH Optimization Progress

Track FPS improvements across different optimization stages:

<script setup>
import PerformanceChart from '../.vitepress/components/PerformanceChart.vue'
import TraversalChart from '../.vitepress/components/TraversalChart.vue'
</script>

<PerformanceChart type="bar" title="BVH Optimization: FPS Over Time" />

## Traversal Complexity

Comparing algorithmic complexity of different acceleration structures:

<TraversalChart />

## Performance Breakdown

Built-in Mermaid pie chart showing where time is spent during ray tracing:

```mermaid
%%{init: {'theme':'dark'}}%%
pie title "Ray Tracing Time Distribution"
    "BVH Traversal" : 45
    "Triangle Intersection" : 30
    "Shading Computation" : 15
    "Memory Bandwidth" : 10
```

## Construction Algorithm Comparison

```mermaid
%%{init: {'theme':'dark'}}%%
graph TD
    A[Scene Geometry] --> B{Algorithm Choice}

    B -->|Speed Priority| C[Linear BVH<br/>~0.5ms]
    B -->|Quality Priority| D[SAH Binned<br/>~2ms]
    B -->|Best Quality| E[SBVH<br/>~50ms]

    C --> F{Use Case}
    D --> F
    E --> F

    F -->|Real-time| G[Linear or SAH]
    F -->|Pre-compute| H[SBVH]

    style C fill:#90EE90,color:#000
    style D fill:#FFD700,color:#000
    style E fill:#FF6B6B,color:#fff
    style G fill:#4FC3F7,color:#000
    style H fill:#9C27B0,color:#fff
```

## Cache Performance Flow

Understanding memory access patterns:

```mermaid
sequenceDiagram
    participant Ray
    participant L1 as L1 Cache
    participant L2 as L2 Cache
    participant L3 as L3 Cache
    participant RAM as Main Memory

    Ray->>L1: Request BVH Node
    alt Cache Hit (90%)
        L1->>Ray: Return (4 cycles)
    else Cache Miss
        L1->>L2: Request
        alt L2 Hit (7%)
            L2->>Ray: Return (12 cycles)
        else L2 Miss
            L2->>L3: Request
            alt L3 Hit (2.5%)
                L3->>Ray: Return (40 cycles)
            else L3 Miss
                L3->>RAM: Request
                RAM->>Ray: Return (100+ cycles)
            end
        end
    end

    Note over Ray,RAM: Coherent traversal = 95% L1 hit rate<br/>Random access = 60% L1 hit rate
```

## Real-World Benchmark Data

### Metrics Table

| Scene | Triangles | BVH Build | Traversal Steps | FPS (1080p) |
|-------|-----------|-----------|-----------------|-------------|
| Sponza | 262K | 2.3ms | 18 avg | 165 |
| Bistro | 2.1M | 18ms | 23 avg | 95 |
| San Miguel | 10M | 95ms | 28 avg | 45 |
| Moana Island | 146M | 1.2s | 35 avg | 8 |

### Build Time vs Quality

```mermaid
%%{init: {'theme':'dark'}}%%
quadrantChart
    title BVH Construction Algorithm Trade-offs
    x-axis Low Quality --> High Quality
    y-axis Slow Build --> Fast Build
    quadrant-1 Fast & Good
    quadrant-2 Fast & Poor
    quadrant-3 Slow & Poor
    quadrant-4 Slow & Good
    Linear: [0.3, 0.9]
    Morton: [0.35, 0.85]
    SAH 8-bin: [0.65, 0.7]
    SAH 16-bin: [0.75, 0.55]
    SAH Full: [0.9, 0.2]
    SBVH: [0.95, 0.1]
```

## Next Steps

- [AABB](/interesting/aabb) - Understanding bounding boxes
- [BVH](/interesting/bvh) - Full BVH implementation details
- [Performance Analysis](/interesting/performance) - Detailed performance guide
