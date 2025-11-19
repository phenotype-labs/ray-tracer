# Game Engine Facts

ray-tracing-engine, performance-optimization, rendering-pipeline, physically-based-rendering

A comprehensive collection of practical facts and implementation details for ray tracing engines. These are battle-tested insights from real-world engine development, focusing on what actually matters for performance and correctness.

## Core Ray Tracing Concepts

### Ray Structure & Properties
1. Rays need **origin point** (P₀) and **direction vector** (d) - both Vec3
2. Direction vector **must be normalized** before intersection tests
3. Ray equation: `P(t) = P₀ + t·d` where t ≥ 0
4. Parameter t represents distance along ray from origin
5. Camera rays start from camera position, **not the screen plane**
6. Screen/viewport is the target plane, not the ray origin
7. Field of view (FOV) affects ray direction spread, not origin
8. Wider FOV = larger angle between adjacent pixel rays
9. Rays can miss everything - always handle null/None intersections
10. Keep track of **closest hit** by maintaining minimum t value
11. Ray has minimum distance (t_min) to prevent self-intersection
12. Ray has maximum distance (t_max) to avoid far plane artifacts
13. Typical t_min = 0.001 (epsilon), t_max = 1000.0 or infinity
14. Ray.at(t) helper function: returns point at distance t along ray
15. Store ray as struct with origin, direction, and optional t_min/t_max
16. Every pixel shoots at least one primary ray into scene
17. Secondary rays (reflection, refraction, shadow) follow same rules
18. Ray direction affects all distance calculations - keep normalized
19. Unnormalized rays break lighting calculations and hit distances
20. Ray casting = single bounce, ray tracing = multiple bounces
21. Pre-calculate ray direction inverse for optimization: `1.0 / ray.direction`
22. Ray differential tracking helps with texture mip-mapping
23. Store ray generation time for motion blur calculations

### Intersection Testing Hierarchy
1. Ray-sphere intersection is **easiest** - quadratic equation solution
2. Sphere test: solve `||P₀ + t·d - C||² = r²` for t
3. Discriminant `b² - 4ac` determines hit count: <0 miss, =0 tangent, >0 two hits
4. Ray-plane intersection is **fundamental** - single dot product
5. Plane test: `t = -(P₀·n + d) / (d·n)` where n is normal, d is distance
6. Ray-triangle uses **Möller-Trumbore algorithm** - one-pass solution
7. Barycentric coordinates (u, v, w) determine hit point within triangle
8. Ray-AABB uses **slab method** - test against 3 axis-aligned planes
9. AABB optimization: 6 plane tests reduced to 3 with min/max operations
10. Implement ray-plane first to understand normal vectors and distances
11. Use bounding volumes to skip expensive triangle tests
12. Early exit when ray misses bounding volume - saves CPU cycles
13. Sort intersection tests by probability: AABB → sphere → triangle
14. Backface culling: skip triangles facing away from ray
15. Test for parallel rays: `|d·n| < epsilon` means ray parallel to plane
16. Handle edge cases: ray origin on surface, grazing angles
17. Pre-compute triangle normals for static geometry
18. Store inverse transformation matrices for faster object-space tests
19. Use squared distance comparisons when possible (avoid sqrt)
20. Möller-Trumbore returns barycentric coords for normal interpolation
21. Ray-cylinder and ray-cone need quadratic solving like spheres
22. Implicit surface intersections: solve f(P) = 0 along ray
23. CSG (Constructive Solid Geometry) combines primitive intersections

```rust
// Example: Tracking closest hit
let mut closest_t = f32::INFINITY;
let mut closest_hit: Option<HitRecord> = None;

for object in scene.objects() {
    if let Some(hit) = object.intersect(ray, 0.001, closest_t) {
        closest_t = hit.t;
        closest_hit = Some(hit);
    }
}
```

## Performance & Optimization

### Spatial Acceleration Structures
1. **BVH (Bounding Volume Hierarchy) is essential** - reduces O(n) to O(log n)
2. Build BVH only when scene changes, not every frame
3. Use AABB (Axis-Aligned Bounding Boxes) for BVH nodes
4. Test ray against BVH node before testing child objects
5. **SAH (Surface Area Heuristic)** determines optimal BVH split planes
6. SAH cost function: `C = C_traverse + Σ(P_i × C_intersect_i)`
7. Median split is fast but produces unbalanced trees
8. SAH split minimizes expected ray traversal cost
9. BVH build strategies: top-down (fast), bottom-up (better quality)
10. SBVH (Spatial BVH) allows primitive splitting for tighter bounds
11. Refit BVH for dynamic scenes instead of full rebuild
12. Refitting updates bounds without changing topology - 10x faster
13. Leaf nodes should contain 1-4 primitives for optimal performance
14. Too few primitives per leaf = deep tree = more traversals
15. Too many primitives per leaf = more intersection tests
16. BVH memory layout: breadth-first for cache coherency
17. Store BVH nodes as 32-byte aligned structs for SIMD
18. Octree works well for sparse scenes with even distribution
19. KD-tree can split space better but more expensive to build
20. Grid acceleration: fast build but poor for uneven distributions
21. Hybrid approaches: grid for primary, BVH for secondary rays
22. GPU BVH traversal benefits from width over depth
23. TLAS (Top-Level AS) + BLAS (Bottom-Level AS) for instancing

### Cache-Friendly Optimizations
1. Sort objects by material to reduce state changes
2. **Precompute inverse matrices** for transformations
3. Early exit ray tests when hit is closer than current closest
4. Use spatial hashing for dynamic objects
5. Implement frustum culling even in ray tracers
6. **Cache ray direction calculations** when possible
7. Store frequently accessed data together (spatial locality)
8. Align data structures to cache line boundaries (64 bytes)
9. Prefetch BVH nodes before traversal
10. Use branch prediction hints for likely ray hits
11. Avoid virtual function calls in hot loops
12. Inline small functions (ray.at(), dot product, etc.)
13. Use texture atlases to reduce texture bind changes
14. Pool allocations instead of per-ray mallocs
15. Reuse ray buffers across frames
16. Sort rays by direction for coherent memory access
17. Batch ray tracing by tile for cache warmth
18. Use bit flags instead of booleans to pack data
19. Compress BVH nodes: quantize bounds to 16-bit
20. Store only essential data in hot structures
21. Lazy evaluation: compute values only when needed
22. Memoize expensive calculations (sky color lookup)
23. Use lookup tables for transcendental functions

### Memory Layout Matters
1. **Array-of-structures (AoS)** vs **Structure-of-arrays (SoA)**
2. AoS: `[{x,y,z,w}, {x,y,z,w}]` - good for single entity access
3. SoA: `{[x,x,x], [y,y,y], [z,z,z]}` - excellent for SIMD
4. SoA with SIMD processes 100k transforms at 2ms per frame
5. Cache misses kill performance more than algorithm complexity
6. L1 cache: ~4 cycles, L2: ~10 cycles, L3: ~40 cycles, RAM: ~200 cycles
7. Cache line = 64 bytes = 16 floats = 4 Vec4s
8. False sharing: avoid writing to same cache line from different threads
9. Dynamic AABB tree with SAH heuristics scales logarithmically
10. Flat arrays outperform linked lists by 10x due to cache
11. Hot/cold data separation: frequently used vs rarely used
12. Quaternion slerp cache misses bottleneck animation
13. Switched from AoS to SoA → 3x performance improvement
14. SIMD intrinsics (AVX2, NEON) process 4-8 values in parallel
15. Vectorize ray-AABB tests: test 4 AABBs simultaneously
16. Structure padding wastes cache space - pack carefully
17. Use `#[repr(C)]` or `#[repr(packed)]` in Rust for control
18. GPU prefers coalesced memory access patterns
19. Texture cache optimized for 2D spatial locality
20. Z-order curve (Morton code) improves spatial locality
21. Bin objects by grid cell for better coherency
22. Double buffering prevents read-write hazards
23. Ring buffers for streaming data reduce allocations

## Lighting & Shadows

### Shadow Ray Fundamentals
1. Shadow rays start from hit point toward light source
2. **Add epsilon offset to avoid self-shadowing (shadow acne)**
3. Typical epsilon: 0.001 - 0.0001 depending on scene scale
4. Offset along surface normal: `shadow_origin = hit_point + normal * epsilon`
5. Shadow rays only need boolean result (hit/no hit) - early exit
6. Check if point is in shadow before calculating lighting contribution
7. Shadow ray length = distance to light (for point/spot lights)
8. Directional lights use infinite shadow ray distance
9. Stop shadow ray at light distance to avoid false occlusion
10. Epsilon too small = shadow acne, epsilon too large = peter panning
11. Peter panning: shadows detach from objects due to large epsilon
12. Use adaptive epsilon based on distance: `epsilon * distance`
13. Normal bias prevents acne, but can miss contact shadows
14. Ray offset bias: move origin along ray direction instead
15. Dual-sided epsilon: offset both toward and away from normal
16. Shadow terminator problem: geometric normal vs shading normal
17. Clamp dot product to prevent negative lighting contributions
18. Shadow rays account for most rays in typical scenes
19. Occluder culling: skip shadow rays for invisible lights
20. Store shadow ray results in cache for reuse across bounces
21. Optimized shadow rays: test closest occluder only
22. Multiple lights = multiple shadow rays per hit point
23. Shadow maps work for primary rays but not global illumination

### Light Types & Sampling
1. **Point lights**: Single shadow ray per light
2. Point light: omnidirectional, position-based
3. **Spot lights**: Cone angle limits illumination region
4. Spot light falloff: smooth edge with cosine interpolation
5. **Directional lights**: No distance falloff, simulates sun
6. Directional light: all shadow rays parallel
7. **Area lights**: Multiple shadow rays create soft shadows
8. Area light sampling: random points on light surface
9. Sphere lights: uniform sampling requires careful distribution
10. Rectangular area lights: simple UV sampling
11. Disk lights: rejection sampling or polar coordinates
12. Triangle lights: barycentric coordinate sampling
13. **Emissive materials** act as area lights naturally
14. Environment/sky lights: sample HDRI texture
15. IES light profiles: photometric data for realistic fixtures
16. Store light positions in world space, not local
17. Light intensity measured in lumens or candelas
18. Light color affects shadow color for colored lights
19. Negative lights subtract illumination (non-physical but useful)
20. Light groups: toggle sets of lights for artistic control
21. Volumetric lights scatter through participating media
22. Portal lights focus sampling on indirect illumination
23. Adaptive light sampling: more samples for bright lights

### Light Physics
1. **Inverse square law**: `intensity = power / (4π × distance²)`
2. Physical light falloff maintains energy conservation
3. Distance = 0 causes division by zero - clamp minimum distance
4. Ambient occlusion: hemisphere sampling for indirect lighting
5. AO rays shorter than global illumination rays
6. Fresnel effect: reflectance increases at grazing angles
7. Schlick's approximation: `F = F₀ + (1 - F₀)(1 - cos θ)⁵`
8. Lambert's cosine law: `intensity × dot(normal, light_dir)`
9. Dot product negative means light behind surface - clamp to zero
10. Attenuation formula: `1 / (constant + linear×d + quadratic×d²)`
11. Constant attenuation: prevents complete darkness at distance
12. Linear attenuation: gradual falloff
13. Quadratic attenuation: physically accurate inverse square
14. Shadow softness proportional to light size / distance ratio
15. Umbra: fully shadowed region (no light rays reach)
16. Penumbra: partially shadowed region (some rays reach)
17. Contact hardening: shadows sharper near contact points
18. Multiple scattering in shadows creates colored bounce light
19. Subsurface scattering allows light through thin objects
20. Caustics: focused light through refractive surfaces
21. God rays: volumetric light shafts through atmosphere
22. Light probe: captures incoming radiance for IBL
23. Spherical harmonics encode low-frequency lighting efficiently

```rust
// Shadow ray with epsilon offset
fn cast_shadow_ray(hit_point: Vec3, light_pos: Vec3, normal: Vec3) -> bool {
    let to_light = (light_pos - hit_point).normalize();
    let shadow_origin = hit_point + normal * EPSILON; // Prevent acne

    let shadow_ray = Ray::new(shadow_origin, to_light);
    let distance_to_light = (light_pos - hit_point).length();

    !scene.any_intersection(shadow_ray, EPSILON, distance_to_light)
}
```

## Materials & Reflections

### Material Properties
1. Store material properties with each object or in material buffer
2. **Diffuse materials** scatter light randomly (Lambertian BRDF)
3. Lambertian: `BRDF = albedo / π` for energy conservation
4. **Specular reflection**: angle of incidence = angle of reflection
5. Reflection formula: `R = I - 2(I·N)N` where I is incident, N is normal
6. Perfect mirrors reflect ray exactly with no scattering
7. Glossy/rough surfaces add random perturbation to reflection
8. Roughness parameter controls reflection lobe spread
9. Metallic materials absorb and re-emit colored light
10. Metals have colored reflections (gold = yellow, copper = orange)
11. Dielectrics (non-metals) have colorless reflections
12. **PBR materials**: albedo, metallic, roughness, normal, AO maps
13. Albedo = base color in linear space (no lighting baked in)
14. Metallic = 0 for dielectric, 1 for metal, rarely in between
15. Roughness = 0 for mirror, 1 for completely diffuse
16. F₀ (base reflectivity): ~0.04 for dielectrics, 0.5-1.0 for metals
17. Anisotropic materials: different roughness in tangent/bitangent
18. Clearcoat layer: additional specular layer on top (car paint)
19. Sheen: retroreflective fuzzy layer (fabric, velvet)
20. Subsurface scattering: light enters and exits at different points
21. Emissive materials: self-illuminating surfaces
22. Transmission: light passes through (glass, water)
23. Thin-film interference: rainbow effect on soap bubbles, oil

### Reflection Behavior
1. Fresnel effect: reflectance increases at grazing angles
2. At normal incidence: metals reflect 50-90%, dielectrics 2-5%
3. At grazing angle (90°): all materials approach 100% reflection
4. Schlick approximation faster than full Fresnel equations
5. Reflection rays follow same tracing rules as primary rays
6. **Limit reflection depth to prevent infinite bounces**
7. Typical max depth: 3-8 bounces depending on performance budget
8. Each bounce reduces ray energy/contribution (throughput)
9. Throughput = accumulated attenuation from all bounces
10. Russian roulette: probabilistically terminate low-energy rays
11. Specular rays offset by epsilon along normal to prevent acne
12. Reflection between parallel mirrors creates infinite depth
13. Path length increases exponentially with bounce count
14. Multi-bounce indirect lighting: light bounces between surfaces
15. Cook-Torrance BRDF: microfacet model for specular reflection
16. GGX/Trowbridge-Reitz: popular normal distribution function
17. Geometry function: accounts for self-shadowing of microfacets
18. Smith geometry function with GGX correlation factor
19. Importance sampling: align rays with BRDF lobe for efficiency
20. Cosine-weighted hemisphere sampling for diffuse materials
21. GGX importance sampling for specular reflections
22. Multiple importance sampling: combine light and BRDF sampling
23. Layered materials: combine multiple BRDFs (plastic = diffuse + specular)

```rust
// Reflection with energy attenuation
fn trace_reflection(ray: Ray, depth: u32, max_depth: u32) -> Color {
    if depth >= max_depth {
        return Color::BLACK;
    }

    if let Some(hit) = scene.intersect(ray) {
        let reflected_dir = reflect(ray.direction, hit.normal);
        let reflected_ray = Ray::new(hit.point + hit.normal * EPSILON, reflected_dir);

        let reflected_color = trace_reflection(reflected_ray, depth + 1, max_depth);
        hit.material.color * reflected_color * hit.material.reflectivity
    } else {
        scene.sky_color(ray.direction)
    }
}
```

## Refraction & Transparency

### Physical Laws
1. Refraction bends rays based on **IOR (index of refraction)**
2. IOR = speed of light in vacuum / speed of light in medium
3. **Snell's law**: `n₁ × sin(θ₁) = n₂ × sin(θ₂)`
4. Air IOR ≈ 1.0, water ≈ 1.33, glass ≈ 1.5, diamond ≈ 2.42
5. Higher IOR = stronger bending of light rays
6. Total internal reflection (TIR) happens at critical angle
7. Critical angle: `θ_c = arcsin(n₂/n₁)` when n₁ > n₂
8. TIR occurs when ray exits denser medium at steep angle
9. Glass needs both reflection and refraction rays blended
10. Fresnel equations determine reflection/refraction split
11. At Brewster's angle: p-polarized light has zero reflection
12. Refraction formula: `T = (n₁/n₂)I + ((n₁/n₂)cosθ₁ - cosθ₂)N`
13. Cosine of refracted angle: `cosθ₂ = sqrt(1 - (n₁/n₂)²sin²θ₁)`
14. Negative discriminant indicates total internal reflection
15. **Beer-Lambert law**: `I = I₀ × e^(-αd)` for absorption
16. Absorption coefficient α determines color tint through medium
17. Thicker glass = more absorption = darker/more colored
18. White glass still absorbs slightly (greenish tint)
19. Dispersion: IOR varies by wavelength (prism rainbow effect)
20. Cauchy equation: `n(λ) = A + B/λ²` models dispersion
21. Chromatic aberration from dispersion separates colors
22. Diamond's high dispersion creates rainbow "fire"
23. Transmission includes both refraction and transparency

### Implementation Details
1. Track whether ray is entering or exiting material
2. Ray enters: dot(incident, normal) < 0
3. Ray exits: dot(incident, normal) > 0
4. Flip normal when ray exits to point outward
5. Track current IOR as ray travels through nested materials
6. IOR stack handles overlapping transparent objects
7. Thin objects (single-sided polygons) need special handling
8. Thin-walled approximation: refract in, then immediately out
9. Offset refracted ray origin to prevent self-intersection
10. Use negative epsilon offset when exiting material
11. Chromatic aberration: trace separate rays per RGB channel
12. Cheap chromatic aberration: single ray with color offset
13. Transparent shadows accumulate color instead of binary occlusion
14. Attenuation color: tint from absorption (Beer's law)
15. Distance through material: track entry to exit point length
16. Nested glass: combine IORs carefully (material stack)
17. Solid vs hollow glass changes IOR transitions
18. Handle coplanar surfaces (zero-thickness) gracefully
19. Alpha blending for alpha-tested transparency (leaves, foliage)
20. Stochastic alpha: probabilistically treat as solid or transparent
21. Refraction roughness: scatter refracted ray for frosted glass
22. Portal rendering: refraction to different scene location
23. Check discriminant before sqrt to avoid NaN in TIR

## Normals & Geometry

### Normal Types & Usage
1. Surface normals must be **unit length** (normalized)
2. Unnormalized normals break lighting calculations
3. **Face normal**: perpendicular to triangle plane
4. Face normal = cross product of two triangle edges
5. **Vertex normals**: averaged face normals of adjacent triangles
6. **Interpolate normals for smooth shading** (Phong shading)
7. Barycentric interpolation blends three vertex normals
8. Flat shading: use face normal for entire triangle
9. Smooth shading: use interpolated vertex normal at hit point
10. Normal maps encode perturbations in tangent space
11. Normal maps are RGB textures: (R,G,B) → (X,Y,Z)
12. Decode normal map: `normal = (rgb × 2 - 1).normalize()`
13. Tangent space: tangent, bitangent, normal (TBN matrix)
14. Calculate tangent and bitangent per triangle from UVs
15. TBN matrix transforms tangent-space normal to world space
16. Mikktspace ensures consistent tangent space across tools
17. **Geometric normal** (face normal): prevents light leaks
18. **Shading normal** (interpolated/mapped): creates smoothness
19. Geometric normal used for ray offsetting to prevent acne
20. Shading normal used for lighting calculations (BRDF)
21. Check consistency: `dot(geometric_normal, shading_normal) > 0`
22. Flip shading normal if inconsistent with geometric normal
23. Height maps displace geometry, normal maps fake it

### Normal Orientation
1. Back-face culling: skip triangles with `dot(ray_dir, normal) > 0`
2. Back-facing test: normal points away from ray origin
3. Flip normal to face ray direction for double-sided materials
4. Double-sided: render both sides of triangle (glass, leaves)
5. Store normals per-vertex for smooth meshes
6. Compute normals at load time for static geometry
7. Recompute normals for dynamic/deformed geometry
8. Weighted normal average: larger triangles influence more
9. Area-weighted vertex normals provide better smoothing
10. Hard edges: duplicate vertices with different normals
11. Normal smoothing groups: selective normal averaging
12. Transform normals with inverse transpose of model matrix
13. Scaling breaks normals if not using inverse transpose
14. Bump mapping: perturb normal based on height gradient
15. Parallax mapping: offset UVs based on view angle + height
16. Offset limiting prevents parallax overshoot at edges
17. Normal flipping for shadow rays: use geometric normal
18. Normal orientation determines inside vs outside of object
19. Consistent winding order: CCW = front-face (right-hand rule)
20. Winding order affects normal direction via cross product
21. Mesh normals provided vs computed: trust source normals
22. Recalculate normals if mesh is non-manifold
23. Volume normals: gradient of signed distance field

```rust
// Smooth normal interpolation (Phong shading)
fn interpolate_normal(v0: Vec3, v1: Vec3, v2: Vec3, barycentric: Vec3) -> Vec3 {
    (v0 * barycentric.x +
     v1 * barycentric.y +
     v2 * barycentric.z).normalize()
}
```

## Color & Tone Mapping

### Color Pipeline
1. **Accumulate color in linear space** (no gamma encoding)
2. All lighting calculations happen in linear color space
3. HDR (High Dynamic Range) allows brightness values > 1.0
4. Scene luminance can range from 0.0001 to 100,000+
5. Tone mapping converts HDR to displayable LDR range [0,1]
6. **Gamma correction at final step** (typically γ=2.2 or sRGB)
7. sRGB curve: γ=2.4 for most values with linear segment near black
8. Clamp colors to [0,1] after tone mapping before display
9. Use floating point (f32/f16) for intermediate calculations
10. Integer color causes banding artifacts in gradients
11. Never store colors in sRGB space during rendering
12. Texture sampling: convert sRGB textures to linear on load
13. Output: convert linear to sRGB before writing to screen
14. Color bleeding: bounced light carries surface albedo
15. Energy conservation: total reflected light ≤ incident light
16. Physically based: albedo values typically 0.02-0.95
17. Pure black (0,0,0) and pure white (1,1,1) rare in nature
18. Charcoal albedo ≈ 0.04, snow ≈ 0.9
19. Fresnel blending mixes diffuse and specular contributions
20. Color space conversion: RGB ↔ XYZ ↔ Lab for accuracy
21. Spectral rendering: wavelength-based instead of RGB
22. Metamers: different spectra perceived as same RGB color
23. Proper alpha compositing: pre-multiplied alpha in linear space

### Tone Mapping Algorithms
1. **Reinhard**: Simple and effective `L_out = L_in / (L_in + 1)`
2. Reinhard normalizes by luminance, preserves color ratios
3. **Extended Reinhard**: `L_out = L_in(1 + L_in/L_white²) / (1 + L_in)`
4. L_white parameter controls white point clipping
5. **ACES (Academy Color Encoding System)**: Industry standard, filmic
6. ACES tone curve: `(x(ax+b)) / (x(cx+d)+e)` with specific coefficients
7. Uncharted 2 (Hable): custom filmic curve from game development
8. GT tonemap: `x / (sqrt(x² + 1)` - smooth, simple
9. **Exposure control** simulates camera EV (exposure value)
10. Exposure = 2^EV, typical range: EV -4 to EV +4
11. Auto-exposure: adapt to scene brightness over time
12. Measure scene luminance: geometric mean or histogram
13. Eye adaptation: exponential interpolation between exposures
14. Bloom: blur bright areas before tone mapping
15. Bloom threshold: extract values above 1.0 (or configurable)
16. Color grading: artistic adjustment post tone mapping
17. LUT (Look-Up Table): 3D texture for color transformations
18. Contrast adjustment: S-curve in shadows/highlights
19. Saturation control: lerp between luminance and color
20. Temperature/tint: shift white balance (warm/cool)
21. Filmic curve: toe (shadows), linear (midtones), shoulder (highlights)
22. Toe compression: lift blacks without crushing detail
23. Shoulder rolloff: gradually approach white without clipping

```rust
// Reinhard tone mapping with gamma correction
fn tone_map(hdr_color: Vec3, exposure: f32) -> Vec3 {
    let exposed = hdr_color * exposure;
    let mapped = exposed / (exposed + Vec3::ONE); // Reinhard
    mapped.powf(1.0 / 2.2) // Gamma correction
}
```

## Sampling & Noise

### Sampling Strategy
1. One ray per pixel = noisy, aliased image
2. **Multiple samples per pixel = antialiasing + noise reduction**
3. Antialiasing: average multiple rays across pixel area
4. Jitter ray positions within pixel for stochastic sampling
5. Use different random seeds for each sample
6. Average all samples to get final pixel color
7. Variance decreases with √n samples (Monte Carlo)
8. 4× samples = 2× less noise, 16× samples = 4× less noise
9. Typical sample counts: 1 (preview), 64-256 (production), 1000+ (final)
10. Sample budget: allocate more to difficult pixels (adaptive)
11. Uniform sampling: equal probability for all directions
12. Non-uniform sampling: focus on important directions
13. Sample clamping: limit maximum sample contribution (firefly removal)
14. Fireflies: extremely bright outlier samples from caustics
15. Pixel reconstruction filter: box, tent, Gaussian, Mitchell
16. Box filter: simple average of all samples
17. Tent filter: weight samples by distance from pixel center
18. Gaussian filter: smooth falloff, reduces aliasing
19. Mitchell-Netravali: balanced sharpness and smoothness
20. Filter width affects sharpness vs antialiasing tradeoff
21. Temporal antialiasing (TAA): reuse samples across frames
22. Spatial-temporal noise: decorrelate samples across space/time
23. Blue noise distribution: perceptually pleasing, avoids patterns

### Advanced Sampling Techniques
1. **Stratified sampling**: subdivide pixel into grid
2. Stratified: guarantees coverage, reduces clumping
3. N×N stratified grid = N² samples with better distribution
4. Jittered grid: add randomness within each stratified cell
5. Multi-jittered sampling: stratified in 1D projections
6. Latin hypercube sampling: stratified in high dimensions
7. **Importance sampling**: align samples with BRDF/light distribution
8. Importance sampling reduces variance dramatically
9. BRDF importance sampling: sample lobe direction
10. Light importance sampling: sample toward light sources
11. **Multiple importance sampling (MIS)**: combine sampling strategies
12. MIS: balance heuristic or power heuristic
13. Balance heuristic: `w_i = (n_i × p_i) / Σ(n_j × p_j)`
14. **Russian roulette**: probabilistically terminate low-contribution paths
15. RR survival probability = max(throughput components), typically clamped
16. RR reduces samples for paths with low expected contribution
17. Never apply RR in first few bounces (introduces bias)
18. **Low-discrepancy sequences**: quasi-random, better coverage
19. Halton sequence: base-p van der Corput for each dimension
20. Sobol sequence: (0,2)-sequence with better 2D projections
21. Hammersley: like Halton but uses sample index for one dimension
22. Blue noise textures: pre-computed importance-sampled patterns
23. Cranley-Patterson rotation: randomize QMC sequences per pixel

```rust
// Stratified sampling within pixel
fn generate_stratified_samples(pixel_x: u32, pixel_y: u32, samples_per_axis: u32) -> Vec<Ray> {
    let mut rays = Vec::new();
    let cell_size = 1.0 / samples_per_axis as f32;

    for i in 0..samples_per_axis {
        for j in 0..samples_per_axis {
            let jitter_x = random_float();
            let jitter_y = random_float();

            let u = (pixel_x as f32 + (i as f32 + jitter_x) * cell_size) / width as f32;
            let v = (pixel_y as f32 + (j as f32 + jitter_y) * cell_size) / height as f32;

            rays.push(camera.generate_ray(u, v));
        }
    }

    rays
}
```

## Debug & Visualization

### Debug Rendering Modes
1. **Visualize normals as RGB colors** - map [-1,1] to [0,1]
2. Normal visualization: red=X, green=Y, blue=Z
3. Render depth buffer to check ray distances
4. Depth buffer helps debug Z-fighting and precision issues
5. Show BVH bounds as wireframe overlay
6. BVH debug: color-code by depth level (red=root, blue=leaves)
7. Color-code different materials for identification
8. Material ID mode: unique color per material
9. Render only primary rays (no bounces) to debug geometry
10. Disable shadows to isolate lighting issues
11. Disable textures to verify base geometry and normals
12. Flat shading mode: use face normals to see triangle mesh
13. UV coordinate visualization: red=U, green=V
14. Barycentric coordinate visualization for triangles
15. Tangent and bitangent visualization for normal mapping
16. Wireframe mode: outline triangles in scene
17. Albedo-only mode: show materials without lighting
18. Roughness/metallic visualization as grayscale
19. Ambient occlusion only: visualize AO contribution
20. Render object IDs for selection and masking
21. Show light positions as colored spheres
22. Visualize light intensity as sphere size/brightness
23. Checkerboard pattern for UV debugging

### Performance Profiling
1. Add wireframe mode to see geometry density
2. Show ray count per pixel as heatmap
3. Heatmap: blue=few rays, red=many rays
4. Identify performance hotspots with ray count visualization
5. Visualize BVH traversal depth per pixel
6. Count intersection tests per ray (performance cost)
7. Visualize shadow rays hitting vs missing lights
8. Sample count visualization: show convergence speed
9. **Debug NaN/infinite values** - they crash/corrupt everything
10. NaN detection: if color.is_nan() return magenta
11. Infinite value detection: if color > threshold return cyan
12. Firefly detection: highlight outlier samples in red
13. Save intermediate buffers for multi-pass analysis
14. Export AOVs (Arbitrary Output Variables): depth, normals, etc.
15. Render passes separately: diffuse, specular, shadows
16. Diff two renders to find differences (regression testing)
17. Adaptive sampling visualization: green=converged, red=noisy
18. Time per pixel heatmap: measure render cost spatially
19. Cache hit/miss visualization for optimization
20. Memory access pattern visualization (cache efficiency)
21. Branch divergence visualization for GPU rays
22. Occupancy heatmap: thread utilization per pixel
23. Frame-to-frame difference for temporal stability

```rust
// Normal visualization for debugging
fn visualize_normal(normal: Vec3) -> Color {
    Color::new(
        (normal.x + 1.0) * 0.5,
        (normal.y + 1.0) * 0.5,
        (normal.z + 1.0) * 0.5
    )
}

// Depth visualization
fn visualize_depth(t: f32, max_depth: f32) -> Color {
    let normalized = (t / max_depth).min(1.0);
    Color::new(normalized, normalized, normalized)
}
```

## Advanced Techniques

### Modern Ray Tracing Methods
1. **Path tracing** = ray tracing with random bounces (Monte Carlo)
2. Path tracing solves rendering equation through Monte Carlo integration
3. Unbiased path tracing: mathematically correct expected value
4. Biased techniques trade accuracy for speed (faster convergence)
5. **Next Event Estimation (NEE)**: explicitly sample lights every bounce
6. NEE dramatically reduces noise for direct lighting
7. Shadow rays from NEE tested for occlusion only
8. Combine NEE with BRDF sampling via MIS for best results
9. **Bidirectional path tracing (BDPT)**: trace from camera and lights
10. BDPT connects light paths and camera paths at vertices
11. BDPT excels at difficult light transport (caustics through glass)
12. Metropolis Light Transport (MLT): MCMC sampling of path space
13. MLT finds important paths and explores nearby mutations
14. **Photon mapping**: cache light photons in spatial structure
15. Photon map first pass: emit photons from lights, store hits
16. Photon map second pass: gather nearby photons at each hit
17. Final gathering: trace rays from photons for quality
18. Progressive photon mapping (PPM): refine photon radius iteratively
19. Vertex Connection and Merging (VCM): unifies BDPT and PPM
20. Light tracing: trace from lights toward camera (reverse path)
21. Manifold exploration: walk along specular manifolds
22. Gradient-domain rendering: render differences for efficiency
23. Primary sample space MLT: better sampling in primary space

### Performance Techniques
1. **Denoising AI** (OIDN, OptiX Denoiser) reduces sample count
2. AI denoiser trains on noisy + converged image pairs
3. Denoise at 16-64 samples instead of 1000+ samples
4. Feature buffers (albedo, normals, depth) guide denoising
5. **Temporal accumulation**: blend current and previous frames
6. Temporal accumulation: `output = mix(history, current, α)`
7. Alpha (blend factor) controls accumulation speed
8. Motion vectors reproject previous frame for moving objects
9. Disocclusion detection: invalidate history when occluder moves
10. Reprojection jitter: temporal antialiasing at 1 SPP
11. Portal rendering: redirect rays to different scene location
12. Portals require recursive scene intersection with transform
13. **Volumetric rendering**: march through participating media
14. Ray marching: sample along ray at regular intervals
15. Transmittance: accumulated extinction along ray `e^(-σt)`
16. In-scattering: light scattered into ray from volume
17. Out-scattering: light scattered out of ray (extinction)
18. Emission: volumetric light emission (fire, plasma)
19. Heterogeneous volumes: varying density (clouds, smoke)
20. Homogeneous volumes: constant density (fog, clear water)
21. **Caustics**: focused light through specular surfaces
22. Caustics need photon mapping or massive sample counts
23. Fake caustics: pre-baked caustic textures projected from light

### Constraint Projection & Physics Stability
1. Constraint projection with Gauss-Seidel iterations
2. Position-based dynamics (PBD): directly correct positions
3. Gauss-Seidel: iteratively satisfy constraints until convergence
4. Jacobi iteration: parallel but slower convergence
5. Proper damping coefficients prevent oscillation
6. Under-damping: bouncy, oscillating motion
7. Over-damping: sluggish, slow settling
8. Critical damping: fastest settling without overshoot
9. Delta-time independent physics: fixed timestep integration
10. Fixed timestep: accumulate dt, step in fixed increments
11. Sub-stepping: multiple small steps per frame for stability
12. Semi-implicit Euler: stable for stiff constraints
13. Verlet integration: implicit velocity, stable for physics
14. SIMD intrinsics for 100k transforms at 2ms/frame
15. AVX2: 8 floats per operation (256-bit registers)
16. NEON: 4 floats per operation (ARM SIMD)
17. Structure-of-arrays enables efficient SIMD vectorization
18. Parallel constraint solving: color graph to avoid conflicts
19. Warm starting: reuse previous frame's solution
20. Contact caching: persist contacts across frames
21. Continuous collision detection (CCD): prevent tunneling
22. Speculative contacts: predict future collisions
23. Shock propagation: iterate constraints from bottom to top

## Facts Summary

### Rays & Intersection
- Ray direction must be normalized for accurate distance calculations
- Maximum ray distance prevents precision issues at infinity
- Epsilon offset (0.001) prevents self-intersection artifacts
- Camera rays use perspective projection, not orthographic
- AABB tests use slab method (6 plane intersections optimized to 3)

### Acceleration & Performance
- BVH reduces intersection tests from O(n) to O(log n)
- Build BVH only when scene changes, not per-frame
- SAH heuristics for AABB tree splits scale logarithmically
- Cache misses from AoS layout kill performance
- SoA with SIMD processes 100k skeletal transforms at 2ms

### Lighting & Materials
- Shadow rays need epsilon offset to prevent self-shadowing
- Inverse square law: `intensity = 1/distance²`
- Fresnel effect increases reflection at grazing angles
- Total internal reflection occurs at steep angles in glass
- Color accumulation happens in linear space, gamma at end

### Sampling & Noise
- Stratified sampling reduces noise better than pure random
- Importance sampling focuses rays where they contribute most
- Low-discrepancy sequences (Halton, Sobol) converge faster
- Russian roulette terminates paths probabilistically
- Temporal accumulation reuses previous frames for stability

### Debug & Visualization
- Normal visualization: map [-1,1] to RGB [0,1]
- Depth visualization: normalize by max depth
- Ray count heatmap identifies performance bottlenecks
- NaN/infinite values cause rendering crashes
- Render primary rays only to debug basic geometry

## Links

- [Core Concepts](./core-concepts) - Ray fundamentals and camera rays
- [Lighting & Shadows](./lighting-shadows) - Shadow rays and light physics
- [Materials & Physics](./materials-physics) - Material properties and reflections
- [Color & Sampling](./color-sampling) - Tone mapping and sampling strategies
- [Advanced Techniques](./advanced-techniques) - Path tracing and modern methods
- [Debug & Visualization](./debug-visualization) - Debug rendering modes
- [BVH](./bvh) - Bounding volume hierarchies
- [AABB](./aabb) - Axis-aligned bounding boxes
