# Ray Tracer Documentation

A high-performance WebGPU-based ray tracer written in Rust, designed for physically-based rendering with glTF support.

## Overview

This documentation covers the core concepts, optimizations, and architecture of the ray tracer. The project focuses on:

- **WebGPU Rendering**: Modern GPU-accelerated ray tracing
- **Physically-Based Rendering (PBR)**: Realistic material and lighting models
- **glTF Support**: Standard 3D asset format (with specific constraints)
- **Performance Optimization**: Advanced spatial acceleration structures and algorithms

## Topics

- **[BVH Visual Guide](BVH_VISUAL_GUIDE.md)**: Understanding Bounding Volume Hierarchies for efficient ray-primitive intersection
- **[Dynamic Worlds](DYNAMIC_WORLDS.md)**: Handling animated scenes and dynamic objects
- **[Temporal Coherence](TEMPORAL_COHERENCE.md)**: Optimizing performance across frames using temporal data

## Project Constraints

- **No glTF cameras**: Programmatic camera control only
- **No glTF light nodes**: Emissive materials serve as area lights (bulbs, screens, etc.)
- **Functional programming**: Clean, DRY, KISS principles throughout

---

**Repository**: [github.com/phenotype-labs/ray-tracer](https://github.com/phenotype-labs/ray-tracer)
