// Simple ray tracer compute shader

struct Camera {
    position: vec3<f32>,
    _pad1: f32,
    forward: vec3<f32>,
    _pad2: f32,
    right: vec3<f32>,
    _pad3: f32,
    up: vec3<f32>,
    _pad4: f32,
};

struct Box {
    min: vec3<f32>,
    _pad1: f32,
    max: vec3<f32>,
    _pad2: f32,
    color: vec3<f32>,
    _pad3: f32,
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
var<storage, read> boxes: array<Box>;

@group(0) @binding(2)
var output_texture: texture_storage_2d<rgba8unorm, write>;

// Ray-box intersection
fn intersect_box(ray: Ray, box: Box) -> HitInfo {
    var hit: HitInfo;
    hit.hit = false;
    hit.distance = 1e10;

    let t_min = (box.min - ray.origin) / ray.direction;
    let t_max = (box.max - ray.origin) / ray.direction;

    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);

    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);

    if t_near > t_far || t_far < 0.0 {
        return hit;
    }

    hit.hit = true;
    hit.distance = select(t_near, t_far, t_near < 0.0);
    hit.position = ray.origin + ray.direction * hit.distance;

    // Calculate normal
    let center = (box.min + box.max) * 0.5;
    let p = hit.position - center;
    let d = abs(p) - (box.max - box.min) * 0.5;
    let bias = 0.001;

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

// Trace ray through scene
fn trace_ray(ray: Ray) -> vec3<f32> {
    var closest_hit: HitInfo;
    closest_hit.hit = false;
    closest_hit.distance = 1e10;

    let num_boxes = arrayLength(&boxes);

    // Check all boxes
    for (var i = 0u; i < num_boxes; i++) {
        let hit = intersect_box(ray, boxes[i]);
        if hit.hit && hit.distance < closest_hit.distance {
            closest_hit = hit;
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

    let final_color = closest_hit.color * (ambient + diffuse * 0.7);

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
