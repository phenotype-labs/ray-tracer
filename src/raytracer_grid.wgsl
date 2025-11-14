// Hierarchical Grid Ray Tracer

const GRID_LEVELS: u32 = 4u;
const MAX_OBJECTS_PER_CELL: u32 = 256u;

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

struct GridMetadata {
    bounds_min: vec3<f32>,
    num_levels: u32,
    bounds_max: vec3<f32>,
    finest_cell_size: f32,
    grid_sizes: array<vec4<u32>, 4>,  // Size of each level (w component is padding)
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

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<uniform> grid_meta: GridMetadata;

@group(0) @binding(2)
var<storage, read> coarse_counts: array<u32>;  // Flattened counts for all coarse levels

@group(0) @binding(3)
var<storage, read> fine_cells: array<FineCellData>;

@group(0) @binding(4)
var<storage, read> boxes: array<Box>;

@group(0) @binding(5)
var output_texture: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(6)
var<uniform> debug_params: DebugParams;

@group(0) @binding(7)
var<storage, read_write> debug_info: RayDebugInfo;

fn should_cull_lod(box_center: vec3<f32>, box_size: vec3<f32>) -> bool {
    let distance = length(camera.position - box_center);
    if distance > 200.0 {
        return true;
    }
    let max_size = max(max(box_size.x, box_size.y), box_size.z);
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

    // If ray origin is inside/behind the box (t_near < 0)
    if t_near < 0.0 {
        // Only return t_far if it's a meaningful distance (not on surface)
        // This prevents self-intersection when ray is on surface pointing out
        if t_far > 0.001 {
            return t_far;
        } else {
            return -1.0;
        }
    }

    return t_near;
}

// Ray-box intersection (detailed hit info)
fn intersect_box(ray: Ray, box: Box, time: f32) -> HitInfo {
    var hit: HitInfo;
    hit.hit = false;
    hit.distance = 1e10;

    // Interpolate box position for moving objects
    // Use sin for oscillating motion between 0 and 1
    let t_lerp = (sin(time * 2.0) + 1.0) * 0.5;
    let interpolated_center = mix(box.center0, box.center1, t_lerp);

    // Use the actual box half-size stored in the structure
    let box_half_size = box.half_size;

    // Create bounds around interpolated center
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

    // Calculate offset to this level in flattened array
    for (var i = 0u; i < level; i++) {
        let size = grid_meta.grid_sizes[i].xyz;
        offset += size.x * size.y * size.z;
    }

    // Add cell index within this level
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

// Check if a world position is near a grid cell boundary
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

    // FIRST: Test moving objects (hardcoded for performance - only 3 in scene)
    // Moving objects need to be tested directly because their AABB in the grid
    // is larger than their actual size at any given time
    let num_boxes = arrayLength(&boxes);
    let moving_start = num_boxes - 3u;
    for (var i = moving_start; i < num_boxes; i++) {
        result.num_steps += 1.0;
        let t_lerp = (sin(camera.time * 2.0) + 1.0) * 0.5;
        let box_center = mix(boxes[i].center0, boxes[i].center1, t_lerp);
        let box_size = boxes[i].half_size * 2.0;

        if !should_cull_lod(box_center, box_size) {
            let hit = intersect_box(ray, boxes[i], camera.time);
            if hit.hit && hit.distance < closest_hit.distance {
                closest_hit = hit;
                result.object_id = f32(i);
            }
        }
    }

    let cell_size = grid_meta.finest_cell_size;
    let grid_size = grid_meta.grid_sizes[GRID_LEVELS - 1u].xyz;

    // Find entry point into grid
    var ray_pos = ray.origin;
    var t_offset = 0.0;  // Offset from ray.origin to ray_pos
    let bounds_min = grid_meta.bounds_min;
    let bounds_max = grid_meta.bounds_max;

    // If ray starts outside grid, find entry point
    if ray_pos.x < bounds_min.x || ray_pos.x > bounds_max.x ||
       ray_pos.y < bounds_min.y || ray_pos.y > bounds_max.y ||
       ray_pos.z < bounds_min.z || ray_pos.z > bounds_max.z {
        // Intersect ray with grid AABB
        let t_entry = intersect_aabb(ray, bounds_min, bounds_max);
        if t_entry < 0.0 {
            // Ray misses grid entirely
            let t = (ray.direction.y + 1.0) * 0.5;
            result.color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
            result.hit = false;
            return result;
        }
        t_offset = t_entry + 0.001;
        ray_pos = ray.origin + ray.direction * t_offset;

        // Clamp ray_pos to be safely inside grid bounds to avoid floating-point precision issues
        // that would cause world_to_cell to clamp negative coordinates to 0
        let epsilon = 0.0001;
        ray_pos = clamp(ray_pos, bounds_min + vec3<f32>(epsilon), bounds_max - vec3<f32>(epsilon));
    }

    // DDA setup
    var current_cell = world_to_cell(ray_pos, cell_size);

    // Step direction (±1 for each axis)
    let step = vec3<i32>(
        select(-1, 1, ray.direction.x >= 0.0),
        select(-1, 1, ray.direction.y >= 0.0),
        select(-1, 1, ray.direction.z >= 0.0)
    );

    // Distance to next cell boundary along each axis
    let cell_pos_world = bounds_min + vec3<f32>(current_cell) * cell_size;
    let next_boundary = cell_pos_world + vec3<f32>(
        select(0.0, cell_size, step.x > 0),
        select(0.0, cell_size, step.y > 0),
        select(0.0, cell_size, step.z > 0)
    );

    // Calculate t values to reach next boundary (relative to ray.origin for correct comparison with hit.distance)
    let t_delta = abs(cell_size / ray.direction);
    var t_max = t_offset + (next_boundary - ray_pos) / ray.direction;

    // Clamp negative values to small positive (handles numerical precision at boundaries)
    t_max = max(t_max, vec3<f32>(t_offset + 0.00001));

    // DDA traversal (max 200 steps to prevent infinite loops)
    for (var i = 0; i < 200; i++) {
        result.num_steps += 1.0;

        // Test objects in current cell
        let fine_idx = get_fine_index(current_cell);
        let cell_data = fine_cells[fine_idx];

        if cell_data.count > 0u {
            for (var j = 0u; j < cell_data.count && j < MAX_OBJECTS_PER_CELL; j++) {
                let obj_idx = cell_data.object_indices[j];
                let box = boxes[obj_idx];
                let box_center = (box.min + box.max) * 0.5;
                let box_size = box.max - box.min;

                if !should_cull_lod(box_center, box_size) {
                    let hit = intersect_box(ray, box, camera.time);
                    if hit.hit && hit.distance < closest_hit.distance {
                        closest_hit = hit;
                        result.object_id = f32(obj_idx);
                    }
                }
            }
        }

        // If we found a hit and it's closer than next cell, stop
        if closest_hit.hit && closest_hit.distance < min(min(t_max.x, t_max.y), t_max.z) {
            break;
        }

        // Step to next cell along shortest t_max, checking bounds before stepping
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
        result.color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
        result.hit = false;
        return result;
    }

    // Simple lighting
    let light_dir = normalize(vec3<f32>(0.5, -1.0, 0.3));
    let diffuse = max(dot(closest_hit.normal, -light_dir), 0.0);
    let ambient = 0.3;

    var final_color = closest_hit.color * (ambient + diffuse * 0.7);

    // Grid visualization
    if camera.show_grid > 0.5 {
        let cell_size = grid_meta.finest_cell_size;
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

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let screen_size = textureDimensions(output_texture);

    // Early exit if outside texture bounds
    if global_id.x >= screen_size.x || global_id.y >= screen_size.y {
        return;
    }

    let pixel_coords = vec2<i32>(global_id.xy);

    // Calculate normalized device coordinates
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

    // Trace the ray with multiple reflection bounces
    const MAX_BOUNCES: u32 = 8u;
    var accumulated_color = vec3<f32>(0.0);
    var current_ray = ray;
    var reflection_multiplier = 1.0;

    // Store first bounce for debug info
    var first_trace_result: TraceResult;

    for (var bounce = 0u; bounce < MAX_BOUNCES; bounce++) {
        let trace_result = trace_ray(current_ray);

        // Store first bounce for debug
        if bounce == 0u {
            first_trace_result = trace_result;
        }

        if !trace_result.hit {
            // Hit sky - add sky color contribution and stop
            accumulated_color += trace_result.color * reflection_multiplier;
            break;
        }

        // Add this surface's diffuse (non-reflected) contribution
        let surface_contribution = trace_result.color * (1.0 - trace_result.reflectivity);
        accumulated_color += surface_contribution * reflection_multiplier;

        // If no reflectivity, we're done
        if trace_result.reflectivity < 0.01 {
            break;
        }

        // Update multiplier for next bounce
        reflection_multiplier *= trace_result.reflectivity;

        // Calculate reflection ray for next bounce
        let reflect_dir = reflect(current_ray.direction, trace_result.normal);
        let reflect_origin = trace_result.position + trace_result.normal * 0.001;

        current_ray.origin = reflect_origin;
        current_ray.direction = reflect_dir;
    }

    var final_color = accumulated_color;

    // Check if this is the debug pixel
    let is_debug_pixel = debug_params.enabled > 0u &&
                         global_id.x == debug_params.debug_pixel.x &&
                         global_id.y == debug_params.debug_pixel.y;

    if is_debug_pixel {
        // Write debug info
        debug_info.ray_origin = ray.origin;
        debug_info.ray_direction = ray.direction;
        debug_info.hit = select(0.0, 1.0, first_trace_result.hit);
        debug_info.distance = first_trace_result.distance;
        debug_info.hit_position = first_trace_result.position;
        debug_info.hit_normal = first_trace_result.normal;
        debug_info.hit_color = first_trace_result.hit_color;
        debug_info.object_id = first_trace_result.object_id;
        debug_info.num_steps = first_trace_result.num_steps;

        // Highlight the debug pixel with a bright border
        final_color = vec3<f32>(1.0, 1.0, 0.0); // Yellow highlight
    } else if debug_params.enabled > 0u {
        // Draw a 3x3 border around the debug pixel
        let dx = i32(global_id.x) - i32(debug_params.debug_pixel.x);
        let dy = i32(global_id.y) - i32(debug_params.debug_pixel.y);
        if (abs(dx) <= 1 && abs(dy) <= 1) && !(dx == 0 && dy == 0) {
            final_color = mix(final_color, vec3<f32>(1.0, 1.0, 0.0), 0.5);
        }
    }

    // Write to output texture
    textureStore(output_texture, pixel_coords, vec4<f32>(final_color, 1.0));
}
