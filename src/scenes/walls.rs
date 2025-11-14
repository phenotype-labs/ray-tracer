use glam::Vec3;
use crate::types::BoxData;
use crate::math::hsv_to_rgb;

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
