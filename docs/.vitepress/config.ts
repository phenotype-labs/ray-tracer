import { defineConfig } from 'vitepress'

export default defineConfig({
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
          text: 'Acceleration Structures',
          items: [
            { text: 'AABB', link: '/interesting/aabb' },
            { text: 'BVH', link: '/interesting/bvh' },
            { text: 'Bounding Spheres', link: '/interesting/bounding-spheres' }
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
  }
})
