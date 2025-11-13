use glam::Vec3;
use crate::types::BoxData;

/// Creates a huge scene to stress test the BVH
pub fn create_default_scene() -> Vec<BoxData> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let ground = BoxData::new([-50.0, -1.0, -50.0], [50.0, -0.99, 50.0], [0.3, 0.3, 0.3]);

    // Dense grid of cubes (20x20 = 400 boxes)
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

    // Floating structures above (15x15x3 = 675 boxes)
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

    // Scattered random boxes (200 boxes)
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

    // Tall pillars on the sides (8x10 = 80 boxes)
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

    // Moving boxes - VERY LARGE and BRIGHT to be impossible to miss
    let moving_boxes = [
        BoxData::create_moving_box(
            Vec3::splat(4.0),
            Vec3::new(0.0, 2.0, -15.0),
            Vec3::new(0.0, 12.0, -15.0),
            [1.0, 0.1, 0.1], // Bright red
        ),
        BoxData::create_moving_box(
            Vec3::splat(3.0),
            Vec3::new(-8.0, 3.0, -12.0),
            Vec3::new(-8.0, 10.0, -12.0),
            [0.1, 1.0, 0.1], // Bright green
        ),
        BoxData::create_moving_box(
            Vec3::splat(3.0),
            Vec3::new(8.0, 3.0, -12.0),
            Vec3::new(8.0, 10.0, -12.0),
            [0.1, 0.1, 1.0], // Bright blue
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
