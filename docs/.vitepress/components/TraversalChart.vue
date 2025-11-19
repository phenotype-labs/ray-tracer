<template>
  <div class="traversal-chart">
    <canvas ref="chartCanvas"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Chart, registerables } from 'chart.js'

Chart.register(...registerables)

const chartCanvas = ref<HTMLCanvasElement | null>(null)

onMounted(() => {
  if (!chartCanvas.value) return

  const ctx = chartCanvas.value.getContext('2d')
  if (!ctx) return

  new Chart(ctx, {
    type: 'line',
    data: {
      labels: ['100', '1K', '10K', '100K', '1M', '10M'],
      datasets: [
        {
          label: 'BVH O(log n)',
          data: [2, 4, 6, 8, 10, 12],
          borderColor: 'rgb(75, 192, 192)',
          backgroundColor: 'rgba(75, 192, 192, 0.2)',
          tension: 0.1
        },
        {
          label: 'Grid O(n) worst case',
          data: [2, 10, 100, 1000, 10000, 100000],
          borderColor: 'rgb(255, 99, 132)',
          backgroundColor: 'rgba(255, 99, 132, 0.2)',
          tension: 0.1
        },
        {
          label: 'Linear O(n)',
          data: [100, 1000, 10000, 100000, 1000000, 10000000],
          borderColor: 'rgb(255, 205, 86)',
          backgroundColor: 'rgba(255, 205, 86, 0.2)',
          tension: 0.1
        }
      ]
    },
    options: {
      responsive: true,
      maintainAspectRatio: true,
      plugins: {
        legend: {
          labels: {
            color: '#fff'
          }
        },
        title: {
          display: true,
          text: 'Traversal Steps vs Primitive Count',
          color: '#fff',
          font: {
            size: 16
          }
        }
      },
      scales: {
        y: {
          type: 'logarithmic',
          ticks: {
            color: '#fff'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.1)'
          },
          title: {
            display: true,
            text: 'Steps (log scale)',
            color: '#fff'
          }
        },
        x: {
          ticks: {
            color: '#fff'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.1)'
          },
          title: {
            display: true,
            text: 'Primitive Count',
            color: '#fff'
          }
        }
      }
    }
  })
})
</script>

<style scoped>
.traversal-chart {
  margin: 2rem 0;
  padding: 1rem;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 8px;
}

canvas {
  max-height: 400px;
}
</style>
