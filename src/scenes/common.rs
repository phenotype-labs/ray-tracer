pub fn should_terminate_fractal(depth: u32, size: f32, min_size: f32) -> bool {
    depth == 0 || size < min_size
}

pub fn generate_fractal_hue(color_seed: u32) -> f32 {
    (color_seed as f32 * 0.618033988749895) % 1.0
}

pub fn generate_fractal_color(color_seed: u32, saturation: f32, value: f32) -> [f32; 3] {
    let hue = generate_fractal_hue(color_seed);
    crate::math::hsv_to_rgb(hue, saturation, value)
}
