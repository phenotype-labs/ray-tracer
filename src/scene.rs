use glam::Vec3;
use crate::types::BoxData;

fn create_menger_sponge(center: Vec3, size: f32, depth: u32, color_seed: u32) -> Vec<BoxData> {
    if depth == 0 || size < 0.3 {
        let half = size * 0.5;
        let hue = (color_seed as f32 * 0.618033988749895) % 1.0;
        let color = hsv_to_rgb(hue, 0.7, 0.8);
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
    if depth == 0 || size < 0.5 {
        let half = size * 0.5;
        let hue = (color_seed as f32 * 0.618033988749895) % 1.0;
        let color = hsv_to_rgb(hue, 0.6, 0.9);
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
    if depth == 0 || size < 0.3 {
        return vec![];
    }

    let mut boxes = Vec::new();
    let half = size * 0.5;
    let hue = (color_seed as f32 * 0.618033988749895) % 1.0;
    let color = hsv_to_rgb(hue, 0.5, 0.7);

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

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let h_prime = (h * 6.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [r + m, g + m, b + m]
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

pub fn create_walls_scene() -> Vec<BoxData> {
    let mut boxes = Vec::new();

    let ground = BoxData::new([-200.0, -1.0, -200.0], [200.0, -0.99, 200.0], [0.15, 0.15, 0.15]);
    boxes.push(ground);

    println!("Generating massive walls scene...");

    let wall_height = 50.0;
    let wall_thickness = 2.0;
    let box_size = 2.0;
    let spacing = 0.2;

    let north_wall_z = -50.0;
    let south_wall_z = 50.0;
    let west_wall_x = -50.0;
    let east_wall_x = 50.0;

    let boxes_per_segment = ((100.0 / (box_size + spacing)) as i32).max(1);
    let boxes_per_height = ((wall_height / (box_size + spacing)) as i32).max(1);

    for layer in 0..boxes_per_height {
        for segment in 0..boxes_per_segment {
            let y = layer as f32 * (box_size + spacing);
            let progress = (segment as f32 / boxes_per_segment as f32 + layer as f32 / boxes_per_height as f32 * 0.3) % 1.0;
            let color = hsv_to_rgb(progress, 0.8, 0.9);

            let x_north = west_wall_x + segment as f32 * (box_size + spacing);
            boxes.push(BoxData::new(
                [x_north, y, north_wall_z - wall_thickness],
                [x_north + box_size, y + box_size, north_wall_z],
                color,
            ));

            let x_south = west_wall_x + segment as f32 * (box_size + spacing);
            boxes.push(BoxData::new(
                [x_south, y, south_wall_z],
                [x_south + box_size, y + box_size, south_wall_z + wall_thickness],
                color,
            ));

            let z_west = north_wall_z + segment as f32 * (box_size + spacing);
            boxes.push(BoxData::new(
                [west_wall_x - wall_thickness, y, z_west],
                [west_wall_x, y + box_size, z_west + box_size],
                color,
            ));

            let z_east = north_wall_z + segment as f32 * (box_size + spacing);
            boxes.push(BoxData::new(
                [east_wall_x, y, z_east],
                [east_wall_x + wall_thickness, y + box_size, z_east + box_size],
                color,
            ));
        }
    }

    for ring in 0..5 {
        let radius = 20.0 + ring as f32 * 5.0;
        let count = 32 + ring * 8;
        let height = 10.0 + ring as f32 * 3.0;

        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            let hue = (i as f32 / count as f32 + ring as f32 * 0.1) % 1.0;
            let color = hsv_to_rgb(hue, 0.7, 0.85);

            boxes.push(BoxData::new(
                [x - 1.5, 0.0, z - 1.5],
                [x + 1.5, height, z + 1.5],
                color,
            ));
        }
    }

    let moving_boxes = [
        BoxData::create_moving_box(
            Vec3::splat(5.0),
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.0, 30.0, 0.0),
            [1.0, 0.3, 0.3],
        ),
        BoxData::create_moving_box(
            Vec3::splat(4.0),
            Vec3::new(-15.0, 8.0, -15.0),
            Vec3::new(15.0, 8.0, 15.0),
            [0.3, 1.0, 0.3],
        ),
        BoxData::create_moving_box(
            Vec3::splat(4.0),
            Vec3::new(15.0, 8.0, -15.0),
            Vec3::new(-15.0, 8.0, 15.0),
            [0.3, 0.3, 1.0],
        ),
    ];
    boxes.extend(moving_boxes);

    println!("Walls scene created: {} total boxes", boxes.len());
    boxes
}

pub fn create_tunnel_scene() -> Vec<BoxData> {
    let mut boxes = Vec::new();

    println!("Generating tunnel scene...");

    let tunnel_length = 300.0;
    let segments = 150;
    let segment_length = tunnel_length / segments as f32;

    for segment in 0..segments {
        let z = -200.0 + segment as f32 * segment_length;
        let progress = segment as f32 / segments as f32;

        let twist = progress * std::f32::consts::TAU * 2.0;
        let radius = 8.0 + (progress * std::f32::consts::TAU * 3.0).sin() * 2.0;
        let sides = 8;

        for side in 0..sides {
            let angle = (side as f32 / sides as f32) * std::f32::consts::TAU + twist;
            let x = angle.cos() * radius;
            let y = angle.sin() * radius;

            let hue = (progress + side as f32 / sides as f32 * 0.3) % 1.0;
            let color = hsv_to_rgb(hue, 0.85, 0.95);

            let box_size = 1.5;
            boxes.push(BoxData::new(
                [x - box_size * 0.5, y - box_size * 0.5, z],
                [x + box_size * 0.5, y + box_size * 0.5, z + segment_length],
                color,
            ));
        }

        if segment % 10 == 0 {
            for ring in 0..3 {
                let ring_radius = radius * 0.3 + ring as f32 * 0.8;
                let ring_sides = 6;

                for side in 0..ring_sides {
                    let angle = (side as f32 / ring_sides as f32) * std::f32::consts::TAU + twist * 0.5;
                    let x = angle.cos() * ring_radius;
                    let y = angle.sin() * ring_radius;

                    let hue = (progress + ring as f32 * 0.15) % 1.0;
                    let color = hsv_to_rgb(hue, 0.7, 0.8);

                    boxes.push(BoxData::new(
                        [x - 0.5, y - 0.5, z - 0.5],
                        [x + 0.5, y + 0.5, z + segment_length + 0.5],
                        color,
                    ));
                }
            }
        }
    }

    let moving_boxes = [
        BoxData::create_moving_box(
            Vec3::splat(3.0),
            Vec3::new(0.0, 0.0, -50.0),
            Vec3::new(0.0, 0.0, -150.0),
            [1.0, 0.2, 0.2],
        ),
        BoxData::create_moving_box(
            Vec3::splat(2.5),
            Vec3::new(3.0, 3.0, -80.0),
            Vec3::new(-3.0, -3.0, -120.0),
            [0.2, 1.0, 0.2],
        ),
        BoxData::create_moving_box(
            Vec3::splat(2.5),
            Vec3::new(-3.0, 3.0, -100.0),
            Vec3::new(3.0, -3.0, -60.0),
            [0.2, 0.2, 1.0],
        ),
    ];
    boxes.extend(moving_boxes);

    println!("Tunnel scene created: {} total boxes", boxes.len());
    boxes
}

pub fn create_default_scene() -> Vec<BoxData> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let ground = BoxData::new([-50.0, -1.0, -50.0], [50.0, -0.99, 50.0], [0.3, 0.3, 0.3]);

    let dense_grid = (-10..10).flat_map(|x| {
        (-10..10).map(move |z| {
            let fx = x as f32 * 1.5;
            let fz = z as f32 * 1.5 - 15.0;
            let size = 0.4;
            let color = [
                ((x + 10) as f32 / 20.0) * 0.8 + 0.2,
                ((z + 10) as f32 / 20.0) * 0.8 + 0.2,
                0.6,
            ];
            BoxData::new(
                [fx - size, -0.5, fz - size],
                [fx + size, 0.5, fz + size],
                color,
            )
        })
    });

    let floating_structures = (-7..8).flat_map(|x| {
        (-7..8).flat_map(move |z| {
            (0..3).map(move |y| {
                let fx = x as f32 * 2.0;
                let fy = y as f32 * 2.0 + 2.0;
                let fz = z as f32 * 2.0 - 10.0;
                let size = 0.35;
                let color = [
                    ((x + 7) as f32 / 15.0) * 0.7 + 0.3,
                    (y as f32 / 3.0) * 0.5 + 0.4,
                    ((z + 7) as f32 / 15.0) * 0.7 + 0.3,
                ];
                BoxData::new(
                    [fx - size, fy - size, fz - size],
                    [fx + size, fy + size, fz + size],
                    color,
                )
            })
        })
    });

    let hasher_builder = RandomState::new();
    let scattered_boxes = (0..200).map(|i| {
        let mut hasher = hasher_builder.build_hasher();
        i.hash(&mut hasher);
        let hash = hasher.finish();

        let x = ((hash % 100) as f32 / 100.0) * 40.0 - 20.0;
        let y = (((hash >> 8) % 100) as f32 / 100.0) * 8.0 - 2.0;
        let z = (((hash >> 16) % 100) as f32 / 100.0) * 40.0 - 30.0;
        let size = (((hash >> 24) % 50) as f32 / 100.0) * 0.4 + 0.2;
        let color = [
            ((hash % 100) as f32 / 100.0) * 0.8 + 0.2,
            (((hash >> 8) % 100) as f32 / 100.0) * 0.8 + 0.2,
            (((hash >> 16) % 100) as f32 / 100.0) * 0.8 + 0.2,
        ];
        BoxData::new(
            [x - size, y - size, z - size],
            [x + size, y + size, z + size],
            color,
        )
    });

    let pillars = [-15.0, 15.0].iter().flat_map(|&side| {
        (-5..5).flat_map(move |z| {
            (0..10).map(move |y| {
                let fz = z as f32 * 2.0 - 10.0;
                let fy = y as f32 * 1.5;
                let size = 0.5;
                let color = if side < 0.0 {
                    [0.8, 0.3, 0.3]
                } else {
                    [0.3, 0.3, 0.8]
                };
                BoxData::new(
                    [side - size, fy - size, fz - size],
                    [side + size, fy + size, fz + size],
                    color,
                )
            })
        })
    });

    let moving_boxes = [
        BoxData::create_moving_box(
            Vec3::splat(4.0),
            Vec3::new(0.0, 2.0, -15.0),
            Vec3::new(0.0, 12.0, -15.0),
            [1.0, 0.1, 0.1],
        ),
        BoxData::create_moving_box(
            Vec3::splat(3.0),
            Vec3::new(-8.0, 3.0, -12.0),
            Vec3::new(-8.0, 10.0, -12.0),
            [0.1, 1.0, 0.1],
        ),
        BoxData::create_moving_box(
            Vec3::splat(3.0),
            Vec3::new(8.0, 3.0, -12.0),
            Vec3::new(8.0, 10.0, -12.0),
            [0.1, 0.1, 1.0],
        ),
    ];

    let boxes: Vec<BoxData> = std::iter::once(ground)
        .chain(dense_grid)
        .chain(floating_structures)
        .chain(scattered_boxes)
        .chain(pillars)
        .chain(moving_boxes)
        .collect();

    println!("Moving boxes added at:");
    println!("  Center: z=-15, moving y: 2->12");
    println!("  Left: x=-8, z=-12, moving y: 3->10");
    println!("  Right: x=8, z=-12, moving y: 3->10");
    println!("Scene created: {} boxes (3 moving)", boxes.len());

    boxes
}

pub fn create_reflected_scene() -> Vec<BoxData> {
    let mut boxes = Vec::new();

    println!("Generating reflected light scene...");

    let room_size = 50.0;
    let wall_thickness = 0.5;
    let reflectivity = 0.8;

    // Floor - highly reflective
    boxes.push(BoxData::new_reflective(
        [-room_size, -room_size, -room_size],
        [room_size, -room_size + wall_thickness, room_size],
        [0.2, 0.2, 0.25],
        reflectivity,
    ));

    // Ceiling - highly reflective
    boxes.push(BoxData::new_reflective(
        [-room_size, room_size - wall_thickness, -room_size],
        [room_size, room_size, room_size],
        [0.25, 0.2, 0.2],
        reflectivity,
    ));

    // North wall - reflective
    boxes.push(BoxData::new_reflective(
        [-room_size, -room_size, -room_size],
        [room_size, room_size, -room_size + wall_thickness],
        [0.2, 0.25, 0.2],
        reflectivity,
    ));

    // South wall - reflective
    boxes.push(BoxData::new_reflective(
        [-room_size, -room_size, room_size - wall_thickness],
        [room_size, room_size, room_size],
        [0.25, 0.25, 0.2],
        reflectivity,
    ));

    // West wall - reflective
    boxes.push(BoxData::new_reflective(
        [-room_size, -room_size, -room_size],
        [-room_size + wall_thickness, room_size, room_size],
        [0.2, 0.2, 0.2],
        reflectivity,
    ));

    // East wall - reflective
    boxes.push(BoxData::new_reflective(
        [room_size - wall_thickness, -room_size, -room_size],
        [room_size, room_size, room_size],
        [0.2, 0.2, 0.2],
        reflectivity,
    ));

    // Central light source - bright and emissive-looking
    let light_size = 4.0;
    boxes.push(BoxData::new_reflective(
        [-light_size, -light_size, -light_size],
        [light_size, light_size, light_size],
        [1.0, 0.95, 0.8],
        0.1,
    ));

    // Add some colorful objects around the room to see reflections
    let object_positions = [
        (Vec3::new(15.0, -10.0, 0.0), [1.0, 0.2, 0.2], 0.3),
        (Vec3::new(-15.0, -10.0, 0.0), [0.2, 1.0, 0.2], 0.3),
        (Vec3::new(0.0, -10.0, 15.0), [0.2, 0.2, 1.0], 0.3),
        (Vec3::new(0.0, -10.0, -15.0), [1.0, 1.0, 0.2], 0.3),
    ];

    for (pos, color, refl) in object_positions {
        let size = 3.0;
        boxes.push(BoxData::new_reflective(
            (pos - Vec3::splat(size)).to_array(),
            (pos + Vec3::splat(size)).to_array(),
            color,
            refl,
        ));
    }

    // Add a few floating spherical-ish objects (using boxes) with varying reflectivity
    for i in 0..8 {
        let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
        let radius = 20.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        let y = (i as f32 * 0.5).sin() * 10.0;

        let hue = i as f32 / 8.0;
        let color = hsv_to_rgb(hue, 0.7, 0.8);
        let refl = 0.4 + (i as f32 / 8.0) * 0.4;

        boxes.push(BoxData::new_reflective(
            [x - 2.0, y - 2.0, z - 2.0],
            [x + 2.0, y + 2.0, z + 2.0],
            color,
            refl,
        ));
    }

    // Add a moving reflective object
    let moving_box = BoxData::create_moving_box(
        Vec3::splat(4.0),
        Vec3::new(0.0, 10.0, 0.0),
        Vec3::new(0.0, -10.0, 0.0),
        [0.9, 0.9, 0.95],
    );
    // Manually set reflectivity for the moving box
    let mut moving_reflective = moving_box;
    moving_reflective.reflectivity = 0.9;
    boxes.push(moving_reflective);

    println!("Reflected scene created: {} total boxes", boxes.len());
    boxes
}
