// Hierarchical Grid Ray Tracer

const GRID_LEVELS: u32 = 4u;
const MAX_OBJECTS_PER_CELL: u32 = 64u;

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
    _pad3: f32,
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
    object_indices: array<u32, 64>,
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

    return select(t_near, t_far, t_near < 0.0);
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
fn trace_ray(ray: Ray) -> vec3<f32> {
    var closest_hit: HitInfo;
    closest_hit.hit = false;
    closest_hit.distance = 1e10;

    // FIRST: Test moving objects (hardcoded for performance - only 3 in scene)
    // Moving objects need to be tested directly because their AABB in the grid
    // is larger than their actual size at any given time
    let num_boxes = arrayLength(&boxes);
    let moving_start = num_boxes - 3u;
    for (var i = moving_start; i < num_boxes; i++) {
        let t_lerp = (sin(camera.time * 2.0) + 1.0) * 0.5;
        let box_center = mix(boxes[i].center0, boxes[i].center1, t_lerp);
        let box_size = boxes[i].half_size * 2.0;

        if !should_cull_lod(box_center, box_size) {
            let hit = intersect_box(ray, boxes[i], camera.time);
            if hit.hit && hit.distance < closest_hit.distance {
                closest_hit = hit;
            }
        }
    }

    let cell_size = grid_meta.finest_cell_size;
    let grid_size = grid_meta.grid_sizes[GRID_LEVELS - 1u].xyz;

    // Find entry point into grid
    var ray_pos = ray.origin;
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
            return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
        }
        ray_pos = ray.origin + ray.direction * (t_entry + 0.001);
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

    // Calculate t values to reach next boundary
    let t_delta = abs(cell_size / ray.direction);
    var t_max = abs((next_boundary - ray_pos) / ray.direction);

    // DDA traversal (max 200 steps to prevent infinite loops)
    for (var i = 0; i < 200; i++) {
        // IMPORTANT: Check bounds BEFORE using current_cell
        // (current_cell might have wrapped to huge value if stepping backwards)
        if current_cell.x >= grid_size.x || current_cell.y >= grid_size.y || current_cell.z >= grid_size.z {
            break;
        }

        // Test objects in current cell (safe: bounds checked above)
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
                    }
                }
            }
        }

        // If we found a hit and it's closer than next cell, stop
        if closest_hit.hit && closest_hit.distance < min(min(t_max.x, t_max.y), t_max.z) {
            break;
        }

        // Step to next cell along shortest t_max
        if t_max.x < t_max.y && t_max.x < t_max.z {
            current_cell.x = u32(i32(current_cell.x) + step.x);
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            current_cell.y = u32(i32(current_cell.y) + step.y);
            t_max.y += t_delta.y;
        } else {
            current_cell.z = u32(i32(current_cell.z) + step.z);
            t_max.z += t_delta.z;
        }
    }

    // If no hit, return sky color
    if !closest_hit.hit {
        let t = (ray.direction.y + 1.0) * 0.5;
        return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
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

    return final_color;
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

    // Trace the ray
    let color = trace_ray(ray);

    // Write to output texture
    textureStore(output_texture, pixel_coords, vec4<f32>(color, 1.0));
}
