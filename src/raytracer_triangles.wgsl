// Triangle-based Ray Tracer with Möller-Trumbore intersection

const GRID_LEVELS: u32 = 4u;
const MAX_TRIANGLES_PER_CELL: u32 = 256u;
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
    texture_index: i32,
    metallic: f32,
    roughness: f32,
    _pad: f32,
};

struct GridMetadata {
    bounds_min: vec3<f32>,
    num_levels: u32,
    bounds_max: vec3<f32>,
    finest_cell_size: f32,
    grid_sizes: array<vec4<u32>, 4>,
};

struct FineCellData {
    triangle_indices: array<u32, 256>,
    count: u32,
    _pad: array<u32, 3>,
};

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct HitInfo {
    hit: bool,
    t: f32,
    position: vec3<f32>,
    normal: vec3<f32>,
    color: vec3<f32>,
    triangle_id: u32,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(2) var<storage, read> materials: array<Material>;
@group(0) @binding(3) var<uniform> grid_meta: GridMetadata;
@group(0) @binding(4) var<storage, read> coarse_counts: array<u32>;
@group(0) @binding(5) var<storage, read> fine_cells: array<FineCellData>;
@group(0) @binding(6) var output_texture: texture_storage_2d<rgba8unorm, write>;

// Ray-Triangle intersection using Möller-Trumbore algorithm
fn intersect_triangle(ray: Ray, tri: Triangle) -> HitInfo {
    var hit: HitInfo;
    hit.hit = false;
    hit.t = 1e30;

    let edge1 = tri.v1 - tri.v0;
    let edge2 = tri.v2 - tri.v0;

    let h = cross(ray.direction, edge2);
    let a = dot(edge1, h);

    // Ray parallel to triangle
    if (abs(a) < EPSILON) {
        return hit;
    }

    let f = 1.0 / a;
    let s = ray.origin - tri.v0;
    let u = f * dot(s, h);

    if (u < 0.0 || u > 1.0) {
        return hit;
    }

    let q = cross(s, edge1);
    let v = f * dot(ray.direction, q);

    if (v < 0.0 || u + v > 1.0) {
        return hit;
    }

    let t = f * dot(edge2, q);

    if (t > EPSILON) {
        hit.hit = true;
        hit.t = t;
        hit.position = ray.origin + ray.direction * t;
        hit.normal = normalize(cross(edge1, edge2));
        return hit;
    }

    return hit;
}

// Convert world position to grid cell coordinates
fn world_to_cell(pos: vec3<f32>, cell_size: f32) -> vec3<i32> {
    let relative = pos - grid_meta.bounds_min;
    return vec3<i32>(
        i32(floor(relative.x / cell_size)),
        i32(floor(relative.y / cell_size)),
        i32(floor(relative.z / cell_size))
    );
}

// Convert cell coordinates to flat index
fn cell_to_index(cell: vec3<i32>, grid_size: vec3<u32>) -> u32 {
    return u32(cell.x) + u32(cell.y) * grid_size.x + u32(cell.z) * grid_size.x * grid_size.y;
}

// Get coarse level count for a cell
fn get_coarse_count(level: u32, cell: vec3<i32>) -> u32 {
    let grid_size = grid_meta.grid_sizes[level].xyz;

    if (cell.x < 0 || cell.y < 0 || cell.z < 0 ||
        u32(cell.x) >= grid_size.x || u32(cell.y) >= grid_size.y || u32(cell.z) >= grid_size.z) {
        return 0u;
    }

    // Calculate offset for this level
    var offset = 0u;
    for (var i = 0u; i < level; i++) {
        let size = grid_meta.grid_sizes[i].xyz;
        offset += size.x * size.y * size.z;
    }

    let idx = cell_to_index(cell, grid_size);
    return coarse_counts[offset + idx];
}

// Trace ray through grid and find closest triangle hit
fn trace_ray(ray: Ray) -> HitInfo {
    var closest_hit: HitInfo;
    closest_hit.hit = false;
    closest_hit.t = 1e30;

    // Start at ray origin cell
    let finest_size = grid_meta.finest_cell_size;
    var current_cell = world_to_cell(ray.origin, finest_size);
    let grid_size = grid_meta.grid_sizes[GRID_LEVELS - 1u].xyz;

    // DDA-style grid traversal
    let ray_sign = sign(ray.direction);
    let inv_dir = 1.0 / ray.direction;

    var step = vec3<i32>(
        select(-1, 1, ray.direction.x > 0.0),
        select(-1, 1, ray.direction.y > 0.0),
        select(-1, 1, ray.direction.z > 0.0)
    );

    for (var iterations = 0u; iterations < 1000u; iterations++) {
        // Check if cell is in bounds
        if (current_cell.x >= 0 && current_cell.y >= 0 && current_cell.z >= 0 &&
            u32(current_cell.x) < grid_size.x &&
            u32(current_cell.y) < grid_size.y &&
            u32(current_cell.z) < grid_size.z) {

            // Check coarse levels first for early rejection
            var has_objects = true;
            for (var level = 0u; level < GRID_LEVELS - 1u; level++) {
                let cell_size = finest_size * f32(1u << (GRID_LEVELS - 1u - level));
                let coarse_cell = world_to_cell(ray.origin + ray.direction * f32(iterations) * finest_size, cell_size);

                if (get_coarse_count(level, coarse_cell) == 0u) {
                    has_objects = false;
                    break;
                }
            }

            // Check fine level
            if (has_objects) {
                let cell_idx = cell_to_index(current_cell, grid_size);
                if (cell_idx < arrayLength(&fine_cells)) {
                    let cell = fine_cells[cell_idx];

                    // Test all triangles in this cell
                    for (var i = 0u; i < min(cell.count, MAX_TRIANGLES_PER_CELL); i++) {
                        let tri_idx = cell.triangle_indices[i];
                        if (tri_idx < arrayLength(&triangles)) {
                            let tri = triangles[tri_idx];
                            var hit = intersect_triangle(ray, tri);

                            if (hit.hit && hit.t < closest_hit.t) {
                                closest_hit = hit;
                                closest_hit.triangle_id = tri_idx;

                                // Get material color
                                let mat_id = u32(tri.material_id);
                                if (mat_id < arrayLength(&materials)) {
                                    let material = materials[mat_id];
                                    closest_hit.color = material.base_color.rgb;
                                } else {
                                    closest_hit.color = vec3(0.7, 0.7, 0.7);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Move to next cell
        let cell_min = grid_meta.bounds_min + vec3<f32>(current_cell) * finest_size;
        let cell_max = cell_min + vec3(finest_size);

        let t_max = select(
            (cell_min - ray.origin) * inv_dir,
            (cell_max - ray.origin) * inv_dir,
            ray.direction > vec3(0.0)
        );

        let min_t = min(t_max.x, min(t_max.y, t_max.z));

        if (t_max.x == min_t) {
            current_cell.x += step.x;
        } else if (t_max.y == min_t) {
            current_cell.y += step.y;
        } else {
            current_cell.z += step.z;
        }

        // If we found a hit and we've passed it, stop
        if (closest_hit.hit && min_t > closest_hit.t) {
            break;
        }

        // If we're way outside bounds, stop
        if (current_cell.x < -10 || current_cell.y < -10 || current_cell.z < -10 ||
            current_cell.x > i32(grid_size.x) + 10 ||
            current_cell.y > i32(grid_size.y) + 10 ||
            current_cell.z > i32(grid_size.z) + 10) {
            break;
        }
    }

    return closest_hit;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(output_texture);
    let pixel = vec2<u32>(global_id.xy);

    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }

    // Generate ray
    let uv = (vec2<f32>(pixel) + vec2(0.5)) / vec2<f32>(dims) * 2.0 - 1.0;
    let aspect = f32(dims.x) / f32(dims.y);

    var ray: Ray;
    ray.origin = camera.position;
    ray.direction = normalize(
        camera.forward +
        camera.right * uv.x * aspect +
        camera.up * uv.y
    );

    // Trace ray
    let hit = trace_ray(ray);

    var final_color = vec3<f32>(0.0);

    if (hit.hit) {
        // Simple shading with directional light
        let light_dir = normalize(vec3(0.5, 1.0, 0.3));
        let diffuse = max(dot(hit.normal, light_dir), 0.0);
        let ambient = 0.3;

        final_color = hit.color * (ambient + diffuse * 0.7);
    } else {
        // Sky gradient
        let t = ray.direction.y * 0.5 + 0.5;
        final_color = mix(vec3(0.5, 0.7, 1.0), vec3(1.0, 1.0, 1.0), t);
    }

    textureStore(output_texture, pixel, vec4(final_color, 1.0));
}
