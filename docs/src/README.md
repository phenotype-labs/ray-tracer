# Ray Tracer Documentation

High-performance WebGPU-based ray tracer written in Rust, designed for physically-based rendering.

## About

This ray tracer focuses on:
- **WebGPU Rendering**: Modern GPU-accelerated ray tracing
- **Physically-Based Rendering (PBR)**: Realistic material and lighting models
- **Performance**: Advanced spatial acceleration structures and algorithms
- **Clean Architecture**: Functional programming, DRY, KISS principles

## Project Constraints

- **No glTF cameras**: Programmatic camera control only
- **No glTF light nodes**: Emissive materials serve as area lights

## Documentation

### Interesting Topics

Deep dives into advanced ray tracing concepts and optimizations:

- **[Bounding Spheres](Interesting/BOUNDING_SPHERES.md)**: Theory, mathematics, and implementation of sphere-based acceleration structures

---

**Repository**: [github.com/phenotype-labs/ray-tracer](https://github.com/phenotype-labs/ray-tracer)
