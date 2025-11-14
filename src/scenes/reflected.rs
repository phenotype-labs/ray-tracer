use glam::Vec3;
use crate::types::BoxData;
use crate::math::hsv_to_rgb;

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
