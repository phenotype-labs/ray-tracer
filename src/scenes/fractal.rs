use glam::Vec3;
use crate::types::BoxData;
use super::common::{should_terminate_fractal, generate_fractal_color};

fn create_menger_sponge(center: Vec3, size: f32, depth: u32, color_seed: u32) -> Vec<BoxData> {
    if should_terminate_fractal(depth, size, 0.3) {
        let half = size * 0.5;
        let color = generate_fractal_color(color_seed, 0.7, 0.8);
        return vec![BoxData::new(
            (center - Vec3::splat(half)).to_array(),
            (center + Vec3::splat(half)).to_array(),
            color,
        )];
    }

    let mut boxes = Vec::new();
    let new_size = size / 3.0;
    let offset = new_size;

    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let empty_count = [x, y, z].iter().filter(|&&v| v == 0).count();
                if empty_count >= 2 {
                    continue;
                }

                let new_center = center + Vec3::new(
                    x as f32 * offset,
                    y as f32 * offset,
                    z as f32 * offset,
                );

                boxes.extend(create_menger_sponge(
                    new_center,
                    new_size,
                    depth - 1,
                    color_seed.wrapping_add((x + y * 3 + z * 9) as u32),
                ));
            }
        }
    }

    boxes
}

fn create_sierpinski_pyramid(center: Vec3, size: f32, depth: u32, color_seed: u32) -> Vec<BoxData> {
    if should_terminate_fractal(depth, size, 0.5) {
        let half = size * 0.5;
        let color = generate_fractal_color(color_seed, 0.6, 0.9);
        return vec![BoxData::new(
            (center - Vec3::splat(half)).to_array(),
            (center + Vec3::splat(half)).to_array(),
            color,
        )];
    }

    let mut boxes = Vec::new();
    let new_size = size * 0.5;
    let offset = new_size * 0.5;

    let positions = [
        Vec3::new(0.0, offset, 0.0),
        Vec3::new(offset, -offset, offset),
        Vec3::new(-offset, -offset, offset),
        Vec3::new(offset, -offset, -offset),
        Vec3::new(-offset, -offset, -offset),
    ];

    for (i, pos) in positions.iter().enumerate() {
        boxes.extend(create_sierpinski_pyramid(
            center + *pos,
            new_size,
            depth - 1,
            color_seed.wrapping_add(i as u32 * 7),
        ));
    }

    boxes
}

fn create_fractal_tree(center: Vec3, size: f32, depth: u32, direction: Vec3, angle: f32, color_seed: u32) -> Vec<BoxData> {
    if should_terminate_fractal(depth, size, 0.3) {
        return vec![];
    }

    let mut boxes = Vec::new();
    let half = size * 0.5;
    let color = generate_fractal_color(color_seed, 0.5, 0.7);

    boxes.push(BoxData::new(
        (center - Vec3::splat(half * 0.3)).to_array(),
        (center + Vec3::new(half * 0.3, half * 2.0, half * 0.3)).to_array(),
        color,
    ));

    if depth > 1 {
        let new_size = size * 0.7;
        let branch_length = size * 1.5;

        let right = direction.cross(Vec3::Y).normalize();
        let up = right.cross(direction).normalize();

        let branches = [
            (up.lerp(right, 0.3).normalize(), 0),
            (up.lerp(-right, 0.3).normalize(), 1),
            (up, 2),
        ];

        for (branch_dir, seed_offset) in branches {
            let new_center = center + branch_dir * branch_length;
            boxes.extend(create_fractal_tree(
                new_center,
                new_size,
                depth - 1,
                branch_dir,
                angle,
                color_seed.wrapping_add(seed_offset * 13),
            ));
        }
    }

    boxes
}

pub fn create_fractal_scene() -> Vec<BoxData> {
    let mut boxes = Vec::new();

    let ground = BoxData::new([-100.0, -1.0, -100.0], [100.0, -0.99, 100.0], [0.2, 0.2, 0.2]);
    boxes.push(ground);

    println!("Generating fractal scene...");

    boxes.extend(create_menger_sponge(Vec3::new(0.0, 5.0, -20.0), 12.0, 3, 0));
    println!("  Menger sponge generated: {} boxes", boxes.len());

    let sierpinski_boxes = create_sierpinski_pyramid(Vec3::new(-25.0, 8.0, -30.0), 16.0, 4, 100);
    println!("  Sierpinski pyramid generated: {} boxes", sierpinski_boxes.len());
    boxes.extend(sierpinski_boxes);

    for i in 0..5 {
        let angle = (i as f32 / 5.0) * std::f32::consts::TAU;
        let radius = 40.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        boxes.extend(create_fractal_tree(
            Vec3::new(x, 0.0, z - 20.0),
            2.0,
            5,
            Vec3::Y,
            0.4,
            200 + i * 50,
        ));
    }
    println!("  Fractal trees generated: {} total boxes", boxes.len());

    for ring in 0..3 {
        let count = 12 + ring * 8;
        let radius = 60.0 + ring as f32 * 20.0;
        let size = 8.0 - ring as f32 * 2.0;
        let depth = 2;

        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            boxes.extend(create_menger_sponge(
                Vec3::new(x, 5.0, z - 20.0),
                size,
                depth,
                300u32.wrapping_add(i * 17).wrapping_add(ring * 100),
            ));
        }
    }
    println!("  Menger rings generated: {} total boxes", boxes.len());

    let moving_boxes = [
        BoxData::create_moving_box(
            Vec3::splat(6.0),
            Vec3::new(0.0, 15.0, -15.0),
            Vec3::new(0.0, 25.0, -15.0),
            [1.0, 0.2, 0.2],
        ),
        BoxData::create_moving_box(
            Vec3::splat(4.0),
            Vec3::new(-15.0, 10.0, -10.0),
            Vec3::new(-15.0, 20.0, -10.0),
            [0.2, 1.0, 0.2],
        ),
        BoxData::create_moving_box(
            Vec3::splat(4.0),
            Vec3::new(15.0, 10.0, -10.0),
            Vec3::new(15.0, 20.0, -10.0),
            [0.2, 0.2, 1.0],
        ),
    ];
    boxes.extend(moving_boxes);

    println!("Fractal scene created: {} total boxes", boxes.len());
    boxes
}
