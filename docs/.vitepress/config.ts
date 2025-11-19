import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(defineConfig({
  title: 'Ray Tracer',
  description: 'High-performance WebGPU-based ray tracer with physically-based rendering',
  base: '/ray-tracer/',

  themeConfig: {
    logo: '/logo.svg',

    nav: [
      { text: 'Home', link: '/' },
      { text: 'Interesting Topics', link: '/interesting/' },
      { text: 'GitHub', link: 'https://github.com/phenotype-labs/ray-tracer' }
    ],

    sidebar: {
      '/interesting/': [
        {
          text: 'Quick Reference',
          items: [
            { text: 'Game Engine Facts', link: '/interesting/game-engine-facts' }
          ]
        },
        {
          text: 'Ray Tracing Fundamentals',
          items: [
            { text: 'Core Concepts', link: '/interesting/core-concepts' },
            { text: 'Lighting & Shadows', link: '/interesting/lighting-shadows' },
            { text: 'Materials & Physics', link: '/interesting/materials-physics' },
            { text: 'Color & Sampling', link: '/interesting/color-sampling' }
          ]
        },
        {
          text: 'Acceleration Structures',
          items: [
            { text: 'AABB', link: '/interesting/aabb' },
            { text: 'BVH', link: '/interesting/bvh' },
            { text: 'Bounding Spheres', link: '/interesting/bounding-spheres' }
          ]
        },
        {
          text: 'Advanced Topics',
          items: [
            { text: 'Advanced Techniques', link: '/interesting/advanced-techniques' },
            { text: 'Debug & Visualization', link: '/interesting/debug-visualization' }
          ]
        },
        {
          text: 'Performance',
          items: [
            { text: 'Performance Analysis', link: '/interesting/performance' },
            { text: 'Interactive Charts', link: '/interesting/charts-demo' }
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/phenotype-labs/ray-tracer' }
    ],

    editLink: {
      pattern: 'https://github.com/phenotype-labs/ray-tracer/edit/main/docs/:path',
      text: 'Edit this page on GitHub'
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2024-present Ihor Herasymovych'
    },

    search: {
      provider: 'local'
    }
  },

  markdown: {
    theme: 'github-dark',
    lineNumbers: true
  },

  // Mermaid configuration
  mermaid: {
    // Mermaid config options
  },
  mermaidPlugin: {
    class: 'mermaid'
  }
}))
