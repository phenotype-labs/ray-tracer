<template>
  <div class="game-scene-demo">
    <div class="explanation">
      <h4>How Bounding Volumes Work</h4>
      <p>
        Watch the golden ray sweep across the scene. <strong>Bounding volumes (dashed circles) provide cheap tests</strong>
        to filter out objects before expensive intersection calculations.
      </p>
    </div>

    <div class="main-demo">
      <div class="demo-panel">
        <canvas ref="canvasWith" width="900" height="500"></canvas>
        <div class="panel-stats">
          <div class="stat-item success">
            <span class="label">🟢 Cheap tests passed (need expensive test):</span>
            <span class="value">{{ expensiveTestsWith }}</span>
          </div>
          <div class="stat-item good">
            <span class="label">🟩 Cheap tests rejected (skipped):</span>
            <span class="value">{{ objectsSkipped }}</span>
          </div>
          <div class="stat-item">
            <span class="label">💰 Cost savings:</span>
            <span class="value">{{ speedup }}x faster</span>
          </div>
        </div>
      </div>
    </div>

    <div class="explanation">
      <h4>Why Spheres for Rotating Objects?</h4>
      <div class="rotation-demo">
        <canvas ref="canvasRotation" width="920" height="300"></canvas>
      </div>
      <div class="rotation-stats">
        <div class="stat-box aabb-cost">
          <div class="stat-label">AABB Rotation Cost</div>
          <div class="stat-value">{{ aabbRotationCost }}µs</div>
          <div class="stat-detail">8 corner transforms + min/max</div>
        </div>
        <div class="stat-box sphere-cost">
          <div class="stat-label">Sphere Rotation Cost</div>
          <div class="stat-value">{{ sphereRotationCost }}µs</div>
          <div class="stat-detail">No recomputation needed!</div>
        </div>
        <div class="stat-box speedup-box">
          <div class="stat-label">Performance Gain</div>
          <div class="stat-value">28.4x</div>
          <div class="stat-detail">From benchmark data</div>
        </div>
      </div>
    </div>

    <div class="controls">
      <button @click="toggleAnimation" class="btn">
        {{ isAnimating ? '⏸ Pause Animation' : '▶ Play Animation' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'

const canvasWith = ref<HTMLCanvasElement | null>(null)
const canvasRotation = ref<HTMLCanvasElement | null>(null)
const isAnimating = ref(true)

const expensiveTestsWith = ref(2)
const objectsSkipped = ref(6)
const speedup = computed(() => {
  const total = expensiveTestsWith.value + objectsSkipped.value
  return (total / expensiveTestsWith.value).toFixed(1)
})

const aabbRotationCost = ref(23.65)
const sphereRotationCost = ref(0.83)

let animationFrame: number | null = null
let rayProgress = 0
let rotationAngle = 0

// Simple objects for demo
const objects = [
  { x: 100, y: 80, w: 40, h: 40, color: '#4ecdc4' },
  { x: 250, y: 120, w: 35, h: 35, color: '#4ecdc4' },
  { x: 150, y: 200, w: 45, h: 30, color: '#ff6b6b' },
  { x: 320, y: 180, w: 40, h: 40, color: '#4ecdc4' },
  { x: 200, y: 300, w: 50, h: 35, color: '#ff6b6b' },
  { x: 350, y: 280, w: 38, h: 38, color: '#4ecdc4' },
  { x: 120, y: 340, w: 42, h: 28, color: '#ff6b6b' },
  { x: 300, y: 350, w: 36, h: 36, color: '#4ecdc4' }
]

const drawScene = (ctx: CanvasRenderingContext2D, withBounds: boolean, rayY: number) => {
  const w = ctx.canvas.width
  const h = ctx.canvas.height

  // Background
  ctx.fillStyle = '#1a1a2e'
  ctx.fillRect(0, 0, w, h)

  // Ray from left
  const rayStartX = 20
  const rayEndX = 20 + rayProgress
  ctx.strokeStyle = '#ffd700'
  ctx.lineWidth = 3
  ctx.beginPath()
  ctx.moveTo(rayStartX, rayY)
  ctx.lineTo(rayEndX, rayY)
  ctx.stroke()

  // Ray origin
  ctx.fillStyle = '#ffd700'
  ctx.beginPath()
  ctx.arc(rayStartX, rayY, 5, 0, Math.PI * 2)
  ctx.fill()

  let testsPerformed = 0
  let cheapTestsRejected = 0

  // Draw objects
  objects.forEach((obj) => {
    const objCenterX = obj.x + obj.w / 2
    const objCenterY = obj.y + obj.h / 2
    const isRayPast = rayEndX > obj.x

    // Check if ray would hit bounding volume
    const boundRadius = Math.sqrt((obj.w / 2) ** 2 + (obj.h / 2) ** 2)
    const rayHitsBounds = Math.abs(rayY - objCenterY) < boundRadius && isRayPast

    // Determine if we test this object
    let shouldTest = false
    let boundsRejected = false

    if (rayHitsBounds && isRayPast) {
      shouldTest = true
      testsPerformed++
    } else if (isRayPast) {
      boundsRejected = true
      cheapTestsRejected++
    }

    // Draw bounding sphere
    ctx.strokeStyle = boundsRejected
      ? 'rgba(0, 255, 0, 0.5)'
      : shouldTest
      ? 'rgba(255, 215, 0, 0.7)'
      : 'rgba(255, 255, 255, 0.2)'
    ctx.lineWidth = 2
    ctx.setLineDash([4, 4])
    ctx.beginPath()
    ctx.arc(objCenterX, objCenterY, boundRadius, 0, Math.PI * 2)
    ctx.stroke()
    ctx.setLineDash([])

    // Draw actual object
    ctx.fillStyle = shouldTest
      ? 'rgba(255, 215, 0, 0.8)'
      : boundsRejected
      ? 'rgba(78, 205, 196, 0.6)'
      : obj.color
    ctx.fillRect(obj.x, obj.y, obj.w, obj.h)

    ctx.strokeStyle = '#fff'
    ctx.lineWidth = 2
    ctx.strokeRect(obj.x, obj.y, obj.w, obj.h)

    // Label if testing
    if (shouldTest && isRayPast) {
      ctx.fillStyle = '#ffd700'
      ctx.font = 'bold 13px monospace'
      ctx.textAlign = 'center'
      ctx.fillText('🟢 PASSED', objCenterX, obj.y - 10)
      ctx.fillText('→ Expensive test', objCenterX, obj.y + obj.h + 22)
    } else if (boundsRejected) {
      ctx.fillStyle = '#0f0'
      ctx.font = 'bold 12px monospace'
      ctx.textAlign = 'center'
      ctx.fillText('🟩 SKIPPED', objCenterX, objCenterY)
    }
  })

  expensiveTestsWith.value = testsPerformed
  objectsSkipped.value = cheapTestsRejected
}

const drawRotationComparison = (ctx: CanvasRenderingContext2D) => {
  const w = ctx.canvas.width
  const h = ctx.canvas.height

  ctx.fillStyle = '#1a1a2e'
  ctx.fillRect(0, 0, w, h)

  const centerY = h / 2
  const rectW = 30
  const rectH = 100

  // Frame 1: 0 degrees
  drawRotatedRect(ctx, 150, centerY, rectW, rectH, 0, 'Frame 1: 0°')

  // Frame 2: 45 degrees
  drawRotatedRect(ctx, 380, centerY, rectW, rectH, rotationAngle, `Frame 2: ${Math.round(rotationAngle)}°`)

  // Frame 3: 90 degrees
  drawRotatedRect(ctx, 610, centerY, rectW, rectH, 90, 'Frame 3: 90°')

  // Annotations
  ctx.fillStyle = '#4ecdc4'
  ctx.font = 'bold 14px monospace'
  ctx.textAlign = 'center'
  ctx.fillText('Sphere: Same radius every frame!', w / 2, 30)

  ctx.fillStyle = '#ff6b6b'
  ctx.fillText('AABB: Must recompute bounds every frame!', w / 2, h - 20)
}

const drawRotatedRect = (
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  angle: number,
  label: string
) => {
  const rad = (angle * Math.PI) / 180

  // Calculate AABB bounds for rotated rect
  const corners = [
    { x: -w / 2, y: -h / 2 },
    { x: w / 2, y: -h / 2 },
    { x: w / 2, y: h / 2 },
    { x: -w / 2, y: h / 2 }
  ]

  const rotated = corners.map(c => ({
    x: c.x * Math.cos(rad) - c.y * Math.sin(rad),
    y: c.x * Math.sin(rad) + c.y * Math.cos(rad)
  }))

  const aabbW = Math.max(...rotated.map(c => c.x)) - Math.min(...rotated.map(c => c.x))
  const aabbH = Math.max(...rotated.map(c => c.y)) - Math.min(...rotated.map(c => c.y))

  // Sphere radius (constant!)
  const sphereRadius = Math.sqrt((w / 2) ** 2 + (h / 2) ** 2)

  // Draw AABB (red, grows with rotation)
  ctx.strokeStyle = '#ff6b6b'
  ctx.lineWidth = 2
  ctx.setLineDash([5, 5])
  ctx.strokeRect(x - aabbW / 2, y - aabbH / 2, aabbW, aabbH)
  ctx.setLineDash([])

  // Draw sphere (cyan, constant)
  ctx.strokeStyle = '#4ecdc4'
  ctx.lineWidth = 2
  ctx.beginPath()
  ctx.arc(x, y, sphereRadius, 0, Math.PI * 2)
  ctx.stroke()

  // Draw actual rotated rectangle
  ctx.save()
  ctx.translate(x, y)
  ctx.rotate(rad)
  ctx.fillStyle = 'rgba(200, 200, 255, 0.8)'
  ctx.fillRect(-w / 2, -h / 2, w, h)
  ctx.strokeStyle = '#fff'
  ctx.lineWidth = 2
  ctx.strokeRect(-w / 2, -h / 2, w, h)
  ctx.restore()

  // Label
  ctx.fillStyle = '#fff'
  ctx.font = '12px monospace'
  ctx.textAlign = 'center'
  ctx.fillText(label, x, y + sphereRadius + 25)
}

const animate = () => {
  if (!isAnimating.value) return

  // Animate ray progress
  rayProgress += 2
  if (rayProgress > 850) rayProgress = 0

  // Animate rotation
  rotationAngle = (rotationAngle + 0.5) % 360

  // Draw main canvas
  if (canvasWith.value) {
    const ctx = canvasWith.value.getContext('2d')
    if (ctx) drawScene(ctx, true, 250)
  }

  // Draw rotation canvas
  if (canvasRotation.value) {
    const ctx = canvasRotation.value.getContext('2d')
    if (ctx) drawRotationComparison(ctx)
  }

  animationFrame = requestAnimationFrame(animate)
}

const toggleAnimation = () => {
  isAnimating.value = !isAnimating.value
  if (isAnimating.value) {
    animationFrame = requestAnimationFrame(animate)
  } else if (animationFrame !== null) {
    cancelAnimationFrame(animationFrame)
  }
}

onMounted(() => {
  animationFrame = requestAnimationFrame(animate)
})

onUnmounted(() => {
  if (animationFrame !== null) {
    cancelAnimationFrame(animationFrame)
  }
})
</script>

<style scoped>
.game-scene-demo {
  margin: 2rem 0;
  padding: 1.5rem;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.explanation {
  margin-bottom: 1.5rem;
}

.explanation h4 {
  color: #4ecdc4;
  margin-bottom: 0.75rem;
  font-size: 18px;
}

.explanation p {
  color: rgba(255, 255, 255, 0.85);
  line-height: 1.6;
  margin: 0;
}

.main-demo {
  margin-bottom: 2rem;
}

.demo-panel {
  background: rgba(0, 0, 0, 0.4);
  padding: 1.5rem;
  border-radius: 8px;
  border: 1px solid rgba(78, 205, 196, 0.2);
}

.demo-panel canvas {
  width: 100%;
  height: auto;
  border-radius: 6px;
  display: block;
}

.panel-stats {
  margin-top: 1.5rem;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
}

.stat-item {
  display: flex;
  justify-content: space-between;
  font-family: monospace;
  font-size: 14px;
  padding: 0.75rem 1rem;
  background: rgba(0, 0, 0, 0.4);
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.stat-item .label {
  color: rgba(255, 255, 255, 0.7);
}

.stat-item .value {
  color: #4ecdc4;
  font-weight: bold;
}

.stat-item.bad .value {
  color: #ff6b6b;
}

.stat-item.good .value {
  color: #4ecdc4;
}

.stat-item.success .value {
  color: #ffd700;
  font-size: 16px;
}

.rotation-demo {
  margin: 1rem 0;
}

.rotation-demo canvas {
  width: 100%;
  height: auto;
  border-radius: 8px;
  display: block;
  background: #1a1a2e;
}

.rotation-stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin-top: 1rem;
}

.stat-box {
  padding: 1rem;
  background: rgba(0, 0, 0, 0.4);
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  text-align: center;
}

.stat-box.aabb-cost {
  border-color: rgba(255, 107, 107, 0.3);
}

.stat-box.sphere-cost {
  border-color: rgba(78, 205, 196, 0.3);
}

.stat-box.speedup-box {
  border-color: rgba(255, 215, 0, 0.3);
}

.stat-label {
  color: rgba(255, 255, 255, 0.6);
  font-size: 11px;
  font-family: monospace;
  margin-bottom: 0.5rem;
  text-transform: uppercase;
}

.stat-value {
  color: #fff;
  font-size: 28px;
  font-weight: bold;
  font-family: monospace;
  margin-bottom: 0.25rem;
}

.stat-box.aabb-cost .stat-value {
  color: #ff6b6b;
}

.stat-box.sphere-cost .stat-value {
  color: #4ecdc4;
}

.stat-box.speedup-box .stat-value {
  color: #ffd700;
}

.stat-detail {
  color: rgba(255, 255, 255, 0.5);
  font-size: 10px;
  font-family: monospace;
}

.controls {
  display: flex;
  justify-content: center;
  margin-top: 1.5rem;
}

.btn {
  padding: 0.75rem 2rem;
  background: rgba(78, 205, 196, 0.2);
  border: 2px solid rgba(78, 205, 196, 0.5);
  border-radius: 8px;
  color: #4ecdc4;
  font-family: monospace;
  font-size: 14px;
  font-weight: bold;
  cursor: pointer;
  transition: all 0.2s;
}

.btn:hover {
  background: rgba(78, 205, 196, 0.3);
  border-color: rgba(78, 205, 196, 0.8);
  transform: translateY(-2px);
}

.btn:active {
  transform: translateY(0);
}

@media (max-width: 968px) {
  .panel-stats {
    grid-template-columns: 1fr;
  }

  .rotation-stats {
    grid-template-columns: 1fr;
  }
}
</style>
