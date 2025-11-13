# How Ray Tracing Works

## Overview

This is a GPU-accelerated ray tracer that renders 3D scenes made of boxes. It uses hierarchical spatial acceleration to efficiently handle thousands of objects.

## Core Concept

**Ray Tracing** = shooting rays from the camera through each pixel to see what they hit.

```
Camera → Ray → Objects in Scene → Calculate Color → Display Pixel
```

## Main Components

### 1. Camera (camera.rs)
- Position in 3D space
- Direction vectors (forward, right, up)
- Keyboard controls: WASD (move), Q/E (rotate), Space/Shift (up/down)
- Generates rays for each pixel based on camera orientation

### 2. Scene (scene.rs)
- Collection of boxes (both static and animated)
- Each box has: position, size, color
- Moving boxes interpolate between two positions over time

### 3. Hierarchical Grid (grid.rs)
- Acceleration structure to avoid testing every ray against every object
- 4 levels of spatial subdivision (coarse → fine)
- **Key strategy:** Each box is assigned to ALL grid cells it overlaps
- **Ray only checks its current cell** (no neighbor checking needed)
- See [GRID_ACCELERATION.md](GRID_ACCELERATION.md) for detailed explanation

### 4. GPU Renderer (renderer.rs + raytracer_grid.wgsl)

**CPU Side:**
- Sets up WebGPU pipelines
- Uploads scene data to GPU buffers
- Updates camera each frame
- Manages display and UI

**GPU Side (Compute Shader):**
- Each pixel runs in parallel
- For each pixel:
  1. Generate ray from camera through pixel
  2. Traverse hierarchical grid to find candidate boxes
  3. Test ray intersection with each candidate
  4. Find closest hit
  5. Calculate color (object color + simple shading)
  6. Write color to output texture

## Ray-Box Intersection

AABB (Axis-Aligned Bounding Box) intersection:
- Calculate where ray enters/exits box on each axis
- Ray hits if all intervals overlap
- Distance = nearest intersection point

## Performance Optimization

**Hierarchical Grid with Box Duplication:**
- Without grid: test ray against all N objects = O(N) per ray
- With grid: test only ~10 objects per cell = O(1) lookup
- Trade-off: Each box stored in multiple cells (uses more memory)
- Benefit: Ray only checks 1 cell at a time (much faster on GPU)
- Result: ~20× speed improvement
- 4 levels: coarse cells for early rejection, fine cells for precise intersection

**GPU Parallelism:**
- All pixels computed simultaneously
- Workgroups of 8×8 pixels
- 800×600 = 480,000 rays per frame in parallel

## Animation

Moving objects:
- Store two positions (center0, center1)
- Interpolate based on time: `pos = mix(center0, center1, sin(time))`
- AABB bounds expanded to cover full movement range
- Ray tests against interpolated position at current frame time

## Rendering Pipeline

1. **Compute Pass**: Ray tracer runs on GPU, writes to texture
2. **Render Pass**: Fullscreen quad displays texture
3. **UI Pass**: Draw FPS counter with egui
4. **Present**: Show final frame on screen

## File Structure

**Code:**
- `main.rs` - Application loop and window management
- `camera.rs` - Camera logic and controls
- `scene.rs` - Scene setup and object definitions
- `types.rs` - Data structures shared between CPU/GPU
- `grid.rs` - Hierarchical grid acceleration structure
- `renderer.rs` - WebGPU setup and rendering coordination
- `raytracer_grid.wgsl` - GPU ray tracing shader code
- `display.wgsl` - Shader to display ray traced image

**Documentation:**
- `HOW_IT_WORKS.md` - High-level overview (you are here)
- `GRID_ACCELERATION.md` - Detailed grid strategy explanation

## Tech Stack

- **Language**: Rust
- **Graphics API**: WebGPU (via wgpu)
- **Math**: glam (vectors, matrices)
- **Windowing**: winit
- **UI**: egui
