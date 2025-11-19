---
layout: home

hero:
  name: Ray Tracer
  text: WebGPU-Based Physically-Based Rendering
  tagline: High-performance ray tracing engine written in Rust
  actions:
    - theme: brand
      text: Get Started
      link: /interesting/
    - theme: alt
      text: View on GitHub
      link: https://github.com/phenotype-labs/ray-tracer

features:
  - icon: ⚡
    title: WebGPU Rendering
    details: Modern GPU-accelerated ray tracing leveraging the latest web graphics APIs
  - icon: 🎨
    title: Physically-Based Rendering
    details: Realistic material and lighting models for photorealistic results
  - icon: 🚀
    title: Performance-Focused
    details: Advanced spatial acceleration structures (BVH, AABB) for real-time performance
  - icon: 🧩
    title: Clean Architecture
    details: Functional programming principles - DRY, KISS, and maintainable code
---

## About

This ray tracer focuses on high-performance real-time rendering using WebGPU. Built with Rust, it implements advanced acceleration structures and physically-based materials.

### Project Constraints

- **No glTF cameras**: Programmatic camera control only
- **No glTF light nodes**: Emissive materials serve as area lights (bulbs, screens, etc.)

### Documentation

Explore our **[Interesting Topics](/interesting/)** section for deep dives into:
- Bounding Volume Hierarchies (BVH)
- Axis-Aligned Bounding Boxes (AABB)
- Surface Area Heuristic (SAH)
- Ray-primitive intersection optimizations
