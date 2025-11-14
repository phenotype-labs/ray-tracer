use glam::Vec3;
use crate::types::BoxData;
use crate::math::hsv_to_rgb;

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
