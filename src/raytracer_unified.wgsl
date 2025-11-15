// Unified Ray Tracer - Supports both triangles and boxes with advanced features
// Based on raytracer_grid.wgsl with triangle intersection from raytracer_triangles.wgsl

const GRID_LEVELS: u32 = 4u;
const MAX_OBJECTS_PER_CELL: u32 = 256u;
const EPSILON: f32 = 0.00001;

struct Camera {
    position: vec3<f32>,
    _pad1: f32,
    forward: vec3<f32>,
    _pad2: f32,
    right: vec3<f32>,
    _pad3: f32,
    up: vec3<f32>,
    time: f32,
    lod_factor: f32,
    min_pixel_size: f32,
    show_grid: f32,
    _pad4: f32,
};

struct Box {
    min: vec3<f32>,
    is_moving: f32,
    max: vec3<f32>,
    _pad2: f32,
    color: vec3<f32>,
    reflectivity: f32,
    center0: vec3<f32>,
    _pad4: f32,
    center1: vec3<f32>,
    _pad5: f32,
    half_size: vec3<f32>,
    _pad6: f32,
};

struct Triangle {
    v0: vec3<f32>,
    material_id: f32,
    v1: vec3<f32>,
    _pad1: f32,
    v2: vec3<f32>,
    _pad2: f32,
    uv0: vec2<f32>,
    uv1: vec2<f32>,
    uv2: vec2<f32>,
    _pad3: vec2<f32>,
};

struct Material {
    base_color: vec4<f32>,
    emissive: vec3<f32>,
    texture_index: i32,
    metallic: f32,
    roughness: f32,
    normal_texture_index: i32,
    emissive_texture_index: i32,
    alpha_mode: u32,  // 0 = OPAQUE, 1 = MASK, 2 = BLEND
    alpha_cutoff: f32,
    _pad: vec2<f32>,
};

struct GridMetadata {
    bounds_min: vec3<f32>,
    num_levels: u32,
    bounds_max: vec3<f32>,
    finest_cell_size: f32,
    grid_sizes: array<vec4<u32>, 4>,
};

struct FineCellData {
    object_indices: array<u32, 256>,
    count: u32,
    _pad: array<u32, 3>,
};

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct HitInfo {
    hit: bool,
    distance: f32,
    position: vec3<f32>,
    normal: vec3<f32>,
    color: vec3<f32>,
    reflectivity: f32,
    object_id: u32,
    is_triangle: bool,
    emissive: vec3<f32>,
    roughness: f32,
};

struct TraceResult {
    color: vec3<f32>,
    hit: bool,
    distance: f32,
    position: vec3<f32>,
    normal: vec3<f32>,
    hit_color: vec3<f32>,
    object_id: f32,
    num_steps: f32,
    reflectivity: f32,
};

struct DebugParams {
    debug_pixel: vec2<u32>,
    enabled: u32,
    _pad: u32,
};

struct RayDebugInfo {
    ray_origin: vec3<f32>,
    hit: f32,
    ray_direction: vec3<f32>,
    distance: f32,
    hit_position: vec3<f32>,
    object_id: f32,
    hit_normal: vec3<f32>,
    num_steps: f32,
    hit_color: vec3<f32>,
    _pad: f32,
};

struct SceneConfig {
    num_boxes: u32,
    num_triangles: u32,
    _pad: vec2<u32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> grid_meta: GridMetadata;
@group(0) @binding(2) var<storage, read> coarse_counts: array<u32>;
@group(0) @binding(3) var<storage, read> fine_cells: array<FineCellData>;
@group(0) @binding(4) var<storage, read> boxes: array<Box>;
@group(0) @binding(5) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(6) var<storage, read> materials: array<Material>;
@group(0) @binding(7) var<uniform> scene_config: SceneConfig;
@group(0) @binding(8) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(9) var<uniform> debug_params: DebugParams;
@group(0) @binding(10) var<storage, read_write> debug_info: RayDebugInfo;
@group(0) @binding(11) var texture_array: texture_2d_array<f32>;
@group(0) @binding(12) var texture_sampler: sampler;

// LOD culling
fn should_cull_lod(object_center: vec3<f32>, object_size: vec3<f32>) -> bool {
    let distance = length(camera.position - object_center);
    if distance > 200.0 {
        return true;
    }
    let max_size = max(max(object_size.x, object_size.y), object_size.z);
    let apparent_size = (max_size / distance) * camera.lod_factor;
    return apparent_size < camera.min_pixel_size;
}

// Ray-AABB intersection
fn intersect_aabb(ray: Ray, box_min: vec3<f32>, box_max: vec3<f32>) -> f32 {
    let t_min = (box_min - ray.origin) / ray.direction;
    let t_max = (box_max - ray.origin) / ray.direction;

    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);

    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);

    if t_near > t_far || t_far < 0.0 {
        return -1.0;
    }

    if t_near < 0.0 {
        if t_far > 0.001 {
            return t_far;
        } else {
            return -1.0;
        }
    }

    return t_near;
}

// Ray-box intersection (detailed hit info)
fn intersect_box(ray: Ray, box: Box, time: f32, box_idx: u32) -> HitInfo {
    var hit: HitInfo;
    hit.hit = false;
    hit.distance = 1e10;
    hit.is_triangle = false;
    hit.object_id = box_idx;

    // Interpolate box position for moving objects
    let t_lerp = (sin(time * 2.0) + 1.0) * 0.5;
    let interpolated_center = mix(box.center0, box.center1, t_lerp);
    let box_half_size = box.half_size;

    let box_min = interpolated_center - box_half_size;
    let box_max = interpolated_center + box_half_size;

    let t = intersect_aabb(ray, box_min, box_max);
    if t < 0.0 {
        return hit;
    }

    hit.hit = true;
    hit.distance = t;
    hit.position = ray.origin + ray.direction * t;

    // Calculate normal using interpolated center
    let p = hit.position - interpolated_center;
    let d = abs(p) - box_half_size;

    if d.x > d.y && d.x > d.z {
        hit.normal = vec3<f32>(sign(p.x), 0.0, 0.0);
    } else if d.y > d.z {
        hit.normal = vec3<f32>(0.0, sign(p.y), 0.0);
    } else {
        hit.normal = vec3<f32>(0.0, 0.0, sign(p.z));
    }

    hit.color = box.color;
    hit.reflectivity = box.reflectivity;
    hit.emissive = vec3<f32>(0.0);
    hit.roughness = 1.0;

    return hit;
}

// Ray-Triangle intersection using Möller-Trumbore algorithm
fn intersect_triangle(ray: Ray, tri: Triangle, tri_idx: u32) -> HitInfo {
    var hit: HitInfo;
    hit.hit = false;
    hit.distance = 1e30;
    hit.is_triangle = true;
    hit.object_id = tri_idx;

    let edge1 = tri.v1 - tri.v0;
    let edge2 = tri.v2 - tri.v0;

    let h = cross(ray.direction, edge2);
    let a = dot(edge1, h);

    // Ray parallel to triangle
    if abs(a) < EPSILON {
        return hit;
    }

    let f = 1.0 / a;
    let s = ray.origin - tri.v0;
    let u = f * dot(s, h);

    if u < 0.0 || u > 1.0 {
        return hit;
    }

    let q = cross(s, edge1);
    let v = f * dot(ray.direction, q);

    if v < 0.0 || u + v > 1.0 {
        return hit;
    }

    let t = f * dot(edge2, q);

    if t > EPSILON {
        hit.hit = true;
        hit.distance = t;
        hit.position = ray.origin + ray.direction * t;
        hit.normal = normalize(cross(edge1, edge2));

        // Get material color
        let mat_id = u32(tri.material_id);
        if mat_id < arrayLength(&materials) {
            let material = materials[mat_id];

            // Calculate UV coordinates using barycentric coordinates
            // u and v from intersection are the barycentric coords for v1 and v2
            let w = 1.0 - u - v;  // barycentric coord for v0
            let uv = tri.uv0 * w + tri.uv1 * u + tri.uv2 * v;

            // Sample base color
            var base_color: vec4<f32>;
            if material.texture_index >= 0 {
                let tex_color = textureSampleLevel(
                    texture_array,
                    texture_sampler,
                    uv,
                    material.texture_index,
                    0.0  // mip level
                );
                base_color = tex_color * material.base_color;
            } else {
                base_color = material.base_color;
            }

            // Handle alpha modes
            if material.alpha_mode == 1u {  // MASK
                if base_color.a < material.alpha_cutoff {
                    return hit;  // Discard this hit
                }
            } else if material.alpha_mode == 2u {  // BLEND
                // For ray tracing, we'll treat blend similar to mask for now
                // Full alpha blending would require sorting and multiple passes
                if base_color.a < 0.01 {
                    return hit;  // Nearly transparent, discard
                }
            }

            hit.color = base_color.rgb;

            // Apply normal mapping if available
            if material.normal_texture_index >= 0 {
                // Sample normal map
                let normal_sample = textureSampleLevel(
                    texture_array,
                    texture_sampler,
                    uv,
                    material.normal_texture_index,
                    0.0
                );

                // Convert from [0,1] to [-1,1] and extract normal
                let tangent_normal = normalize(normal_sample.rgb * 2.0 - 1.0);

                // For simplicity, we'll perturb the geometric normal
                // A full implementation would require tangent/bitangent vectors
                let geometric_normal = normalize(cross(edge1, edge2));

                // Create a simple tangent space (not perfect but works for many cases)
                let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0),
                               abs(geometric_normal.y) > 0.9);
                let tangent = normalize(cross(up, geometric_normal));
                let bitangent = cross(geometric_normal, tangent);

                // Transform normal from tangent space to world space
                let world_normal = tangent_normal.x * tangent +
                                   tangent_normal.y * bitangent +
                                   tangent_normal.z * geometric_normal;
                hit.normal = normalize(world_normal);
            }

            // Add emissive contribution
            var emissive_color = material.emissive;
            if material.emissive_texture_index >= 0 {
                let emissive_sample = textureSampleLevel(
                    texture_array,
                    texture_sampler,
                    uv,
                    material.emissive_texture_index,
                    0.0
                );
                emissive_color *= emissive_sample.rgb;
            }
            hit.emissive = emissive_color;

            // Use metallic as reflectivity and store roughness
            hit.reflectivity = material.metallic;
            hit.roughness = material.roughness;
        } else {
            hit.color = vec3(0.7, 0.7, 0.7);
            hit.reflectivity = 0.0;
            hit.emissive = vec3<f32>(0.0);
            hit.roughness = 1.0;
        }

        return hit;
    }

    return hit;
}

// Convert world position to grid cell coordinates
fn world_to_cell(pos: vec3<f32>, cell_size: f32) -> vec3<u32> {
    let rel_pos = pos - grid_meta.bounds_min;
    return vec3<u32>(
        u32(max(0.0, floor(rel_pos.x / cell_size))),
        u32(max(0.0, floor(rel_pos.y / cell_size))),
        u32(max(0.0, floor(rel_pos.z / cell_size)))
    );
}

// Get flattened index for coarse level
fn get_coarse_index(level: u32, cell: vec3<u32>) -> u32 {
    var offset = 0u;

    for (var i = 0u; i < level; i++) {
        let size = grid_meta.grid_sizes[i].xyz;
        offset += size.x * size.y * size.z;
    }

    let size = grid_meta.grid_sizes[level].xyz;
    return offset + cell.x + cell.y * size.x + cell.z * size.x * size.y;
}

// Get index for fine level cell
fn get_fine_index(cell: vec3<u32>) -> u32 {
    let size = grid_meta.grid_sizes[GRID_LEVELS - 1u].xyz;
    return cell.x + cell.y * size.x + cell.z * size.x * size.y;
}

// Check if cell is within bounds
fn is_cell_valid(cell: vec3<u32>, level: u32) -> bool {
    let size = grid_meta.grid_sizes[level].xyz;
    return cell.x < size.x && cell.y < size.y && cell.z < size.z;
}

// Check if near grid boundary
fn is_near_grid_boundary(pos: vec3<f32>, cell_size: f32, threshold: f32) -> bool {
    let rel_pos = pos - grid_meta.bounds_min;
    let cell_local = (rel_pos % cell_size) / cell_size;

    let dist_x = min(cell_local.x, 1.0 - cell_local.x);
    let dist_y = min(cell_local.y, 1.0 - cell_local.y);
    let dist_z = min(cell_local.z, 1.0 - cell_local.z);

    return dist_x < threshold || dist_y < threshold || dist_z < threshold;
}

// DDA ray marching through grid
fn trace_ray(ray: Ray) -> TraceResult {
    var result: TraceResult;
    result.num_steps = 0.0;
    result.object_id = -1.0;
    result.hit = false;
    result.distance = 1e10;
    result.reflectivity = 0.0;

    var closest_hit: HitInfo;
    closest_hit.hit = false;
    closest_hit.distance = 1e10;

    // Test moving boxes directly (if any)
    let num_boxes = scene_config.num_boxes;
    if num_boxes > 0u {
        // Assume last 3 boxes are moving (hardcoded for performance)
        let moving_start = select(0u, num_boxes - 3u, num_boxes >= 3u);
        for (var i = moving_start; i < num_boxes; i++) {
            result.num_steps += 1.0;
            let t_lerp = (sin(camera.time * 2.0) + 1.0) * 0.5;
            let box_center = mix(boxes[i].center0, boxes[i].center1, t_lerp);
            let box_size = boxes[i].half_size * 2.0;

            if !should_cull_lod(box_center, box_size) {
                let hit = intersect_box(ray, boxes[i], camera.time, i);
                if hit.hit && hit.distance < closest_hit.distance {
                    closest_hit = hit;
                    result.object_id = f32(i);
                }
            }
        }
    }

    let cell_size = grid_meta.finest_cell_size;
    let grid_size = grid_meta.grid_sizes[GRID_LEVELS - 1u].xyz;

    // Find entry point into grid
    var ray_pos = ray.origin;
    var t_offset = 0.0;
    let bounds_min = grid_meta.bounds_min;
    let bounds_max = grid_meta.bounds_max;

    // If ray starts outside grid, find entry point
    if ray_pos.x < bounds_min.x || ray_pos.x > bounds_max.x ||
       ray_pos.y < bounds_min.y || ray_pos.y > bounds_max.y ||
       ray_pos.z < bounds_min.z || ray_pos.z > bounds_max.z {
        let t_entry = intersect_aabb(ray, bounds_min, bounds_max);
        if t_entry < 0.0 {
            // Ray misses grid - return sky
            let t = (ray.direction.y + 1.0) * 0.5;
            result.color = mix(vec3<f32>(0.3, 0.5, 0.7), vec3<f32>(0.5, 0.7, 1.0), t);
            result.hit = false;
            return result;
        }
        t_offset = t_entry + 0.001;
        ray_pos = ray.origin + ray.direction * t_offset;

        let epsilon = 0.0001;
        ray_pos = clamp(ray_pos, bounds_min + vec3<f32>(epsilon), bounds_max - vec3<f32>(epsilon));
    }

    // DDA setup
    var current_cell = world_to_cell(ray_pos, cell_size);

    let step = vec3<i32>(
        select(-1, 1, ray.direction.x >= 0.0),
        select(-1, 1, ray.direction.y >= 0.0),
        select(-1, 1, ray.direction.z >= 0.0)
    );

    let cell_pos_world = bounds_min + vec3<f32>(current_cell) * cell_size;
    let next_boundary = cell_pos_world + vec3<f32>(
        select(0.0, cell_size, step.x > 0),
        select(0.0, cell_size, step.y > 0),
        select(0.0, cell_size, step.z > 0)
    );

    let t_delta = abs(cell_size / ray.direction);
    var t_max = t_offset + (next_boundary - ray_pos) / ray.direction;
    t_max = max(t_max, vec3<f32>(t_offset + 0.00001));

    // DDA traversal
    for (var i = 0; i < 200; i++) {
        result.num_steps += 1.0;

        // Test objects in current cell
        let fine_idx = get_fine_index(current_cell);
        let cell_data = fine_cells[fine_idx];

        if cell_data.count > 0u {
            for (var j = 0u; j < cell_data.count && j < MAX_OBJECTS_PER_CELL; j++) {
                let obj_idx = cell_data.object_indices[j];

                // Check if it's a box or triangle based on index
                // Boxes come first, then triangles
                if obj_idx < num_boxes {
                    // Box
                    let box = boxes[obj_idx];
                    let box_center = (box.min + box.max) * 0.5;
                    let box_size = box.max - box.min;

                    if !should_cull_lod(box_center, box_size) {
                        let hit = intersect_box(ray, box, camera.time, obj_idx);
                        if hit.hit && hit.distance < closest_hit.distance {
                            closest_hit = hit;
                            result.object_id = f32(obj_idx);
                        }
                    }
                } else {
                    // Triangle
                    let tri_idx = obj_idx - num_boxes;
                    if tri_idx < scene_config.num_triangles {
                        let hit = intersect_triangle(ray, triangles[tri_idx], tri_idx);
                        if hit.hit && hit.distance < closest_hit.distance {
                            closest_hit = hit;
                            result.object_id = f32(obj_idx);
                        }
                    }
                }
            }
        }

        // If we found a hit closer than next cell, stop
        if closest_hit.hit && closest_hit.distance < min(min(t_max.x, t_max.y), t_max.z) {
            break;
        }

        // Step to next cell
        if t_max.x < t_max.y && t_max.x < t_max.z {
            let next_x = i32(current_cell.x) + step.x;
            if next_x < 0 || next_x >= i32(grid_size.x) {
                break;
            }
            current_cell.x = u32(next_x);
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            let next_y = i32(current_cell.y) + step.y;
            if next_y < 0 || next_y >= i32(grid_size.y) {
                break;
            }
            current_cell.y = u32(next_y);
            t_max.y += t_delta.y;
        } else {
            let next_z = i32(current_cell.z) + step.z;
            if next_z < 0 || next_z >= i32(grid_size.z) {
                break;
            }
            current_cell.z = u32(next_z);
            t_max.z += t_delta.z;
        }
    }

    // If no hit, return sky color
    if !closest_hit.hit {
        let t = (ray.direction.y + 1.0) * 0.5;
        result.color = mix(vec3<f32>(0.3, 0.5, 0.7), vec3<f32>(0.5, 0.7, 1.0), t);
        result.hit = false;
        return result;
    }

    // Lighting: ambient + directional + emissive surfaces
    let light_dir = normalize(vec3<f32>(0.5, -1.0, 0.3));
    let diffuse = max(dot(closest_hit.normal, -light_dir), 0.0);
    let ambient = 0.3;

    // Direct lighting from emissive surfaces (area lights)
    let emissive_light = sample_emissive_light(closest_hit.position, closest_hit.normal);

    var final_color = closest_hit.color * (ambient + diffuse * 0.7 + emissive_light) + closest_hit.emissive;

    // Grid visualization
    if camera.show_grid > 0.5 {
        let threshold = 0.02;
        if is_near_grid_boundary(closest_hit.position, cell_size, threshold) {
            final_color = mix(final_color, vec3<f32>(0.0, 1.0, 0.0), 0.6);
        }
    }

    result.color = final_color;
    result.hit = true;
    result.distance = closest_hit.distance;
    result.position = closest_hit.position;
    result.normal = closest_hit.normal;
    result.hit_color = closest_hit.color;
    result.reflectivity = closest_hit.reflectivity;

    return result;
}

// Test if a ray is occluded (for shadow rays)
fn is_occluded(ray: Ray, max_distance: f32) -> bool {
    let num_boxes = scene_config.num_boxes;
    let num_triangles = scene_config.num_triangles;

    // Test boxes
    for (var i = 0u; i < num_boxes; i++) {
        let box = boxes[i];
        let box_center = (box.min + box.max) * 0.5;
        let box_size = box.max - box.min;

        if !should_cull_lod(box_center, box_size) {
            let hit = intersect_box(ray, box, camera.time, i);
            if hit.hit && hit.distance < max_distance && hit.distance > 0.001 {
                return true;
            }
        }
    }

    // Test triangles
    for (var i = 0u; i < num_triangles; i++) {
        let hit = intersect_triangle(ray, triangles[i], i);
        if hit.hit && hit.distance < max_distance && hit.distance > 0.001 {
            return true;
        }
    }

    return false;
}

// Calculate direct lighting from emissive surfaces
fn sample_emissive_light(hit_pos: vec3<f32>, hit_normal: vec3<f32>) -> vec3<f32> {
    var total_light = vec3<f32>(0.0);
    let num_triangles = scene_config.num_triangles;

    // Sample emissive triangles
    for (var i = 0u; i < num_triangles; i++) {
        let mat_id = u32(triangles[i].material_id);
        if mat_id >= arrayLength(&materials) {
            continue;
        }

        let material = materials[mat_id];

        // Skip non-emissive materials
        let emissive_strength = length(material.emissive);
        if emissive_strength < 0.001 {
            continue;
        }

        // Calculate triangle center as light position
        let tri = triangles[i];
        let light_pos = (tri.v0 + tri.v1 + tri.v2) / 3.0;
        let light_normal = normalize(cross(tri.v1 - tri.v0, tri.v2 - tri.v0));

        // Vector from hit point to light
        let to_light = light_pos - hit_pos;
        let distance = length(to_light);
        let light_dir = to_light / distance;

        // Check if light is facing the surface
        let n_dot_l = max(dot(hit_normal, light_dir), 0.0);
        if n_dot_l < 0.001 {
            continue;
        }

        // Check if surface is facing the light
        let light_facing = dot(light_normal, -light_dir);
        if light_facing < 0.001 {
            continue;
        }

        // Shadow ray
        var shadow_ray: Ray;
        shadow_ray.origin = hit_pos + hit_normal * 0.001;
        shadow_ray.direction = light_dir;

        if !is_occluded(shadow_ray, distance - 0.002) {
            // Calculate triangle area for proper light intensity
            let edge1 = tri.v1 - tri.v0;
            let edge2 = tri.v2 - tri.v0;
            let area = length(cross(edge1, edge2)) * 0.5;

            // Inverse square falloff with area compensation
            let attenuation = (area * light_facing) / (distance * distance + 1.0);

            total_light += material.emissive * n_dot_l * attenuation;
        }
    }

    return total_light;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let screen_size = textureDimensions(output_texture);

    if global_id.x >= screen_size.x || global_id.y >= screen_size.y {
        return;
    }

    let pixel_coords = vec2<i32>(global_id.xy);

    let uv = vec2<f32>(
        (f32(pixel_coords.x) + 0.5) / f32(screen_size.x),
        (f32(pixel_coords.y) + 0.5) / f32(screen_size.y)
    );

    let ndc = uv * 2.0 - 1.0;

    // Generate ray from camera
    let aspect_ratio = f32(screen_size.x) / f32(screen_size.y);
    let fov_scale = tan(0.785398); // 45 degrees FOV

    let ray_dir = normalize(
        camera.forward +
        camera.right * ndc.x * aspect_ratio * fov_scale +
        camera.up * -ndc.y * fov_scale
    );

    var ray: Ray;
    ray.origin = camera.position;
    ray.direction = ray_dir;

    // Trace with multiple reflection bounces
    const MAX_BOUNCES: u32 = 8u;
    var accumulated_color = vec3<f32>(0.0);
    var current_ray = ray;
    var reflection_multiplier = 1.0;

    var first_trace_result: TraceResult;

    for (var bounce = 0u; bounce < MAX_BOUNCES; bounce++) {
        let trace_result = trace_ray(current_ray);

        if bounce == 0u {
            first_trace_result = trace_result;
        }

        if !trace_result.hit {
            accumulated_color += trace_result.color * reflection_multiplier;
            break;
        }

        // Add diffuse contribution
        let surface_contribution = trace_result.color * (1.0 - trace_result.reflectivity);
        accumulated_color += surface_contribution * reflection_multiplier;

        if trace_result.reflectivity < 0.01 {
            break;
        }

        reflection_multiplier *= trace_result.reflectivity;

        // Calculate reflection ray
        let reflect_dir = reflect(current_ray.direction, trace_result.normal);
        let reflect_origin = trace_result.position + trace_result.normal * 0.001;

        current_ray.origin = reflect_origin;
        current_ray.direction = reflect_dir;
    }

    var final_color = accumulated_color;

    // Debug pixel highlighting
    let is_debug_pixel = debug_params.enabled > 0u &&
                         global_id.x == debug_params.debug_pixel.x &&
                         global_id.y == debug_params.debug_pixel.y;

    if is_debug_pixel {
        debug_info.ray_origin = ray.origin;
        debug_info.ray_direction = ray.direction;
        debug_info.hit = select(0.0, 1.0, first_trace_result.hit);
        debug_info.distance = first_trace_result.distance;
        debug_info.hit_position = first_trace_result.position;
        debug_info.hit_normal = first_trace_result.normal;
        debug_info.hit_color = first_trace_result.hit_color;
        debug_info.object_id = first_trace_result.object_id;
        debug_info.num_steps = first_trace_result.num_steps;

        final_color = vec3<f32>(1.0, 1.0, 0.0);
    } else if debug_params.enabled > 0u {
        let dx = i32(global_id.x) - i32(debug_params.debug_pixel.x);
        let dy = i32(global_id.y) - i32(debug_params.debug_pixel.y);
        if (abs(dx) <= 1 && abs(dy) <= 1) && !(dx == 0 && dy == 0) {
            final_color = mix(final_color, vec3<f32>(1.0, 1.0, 0.0), 0.5);
        }
    }

    textureStore(output_texture, pixel_coords, vec4<f32>(final_color, 1.0));
}
