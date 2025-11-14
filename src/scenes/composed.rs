use glam::Vec3;
use crate::types::BoxData;
use crate::demo::*;
use crate::math::hsv_to_rgb;

/// Example demo scene built using the demo module
/// Showcases the composability and builder pattern
pub fn create_composed_scene() -> Vec<BoxData> {
    DemoBuilder::new()
        // Add a reflective ground
        .add_reflective_ground([0.15, 0.15, 0.2], 0.5)

        // Add concentric rainbow rings
        .add_rings(
            15.0,  // base radius
            8.0,   // radius step
            5,     // number of rings
            |ring| 32 + ring * 8,  // boxes per ring increases with radius
            |ring| Vec3::new(1.5, 8.0 + ring as f32 * 2.0, 1.5),  // height increases
            |ring, i| {
                let total = 32 + ring * 8;
                rainbow_gradient(total)(i)
            }
        )

        // Add a central light source
        .add_reflective_box(
            Vec3::new(0.0, 15.0, 0.0),
            Vec3::splat(4.0),
            [1.0, 0.95, 0.8],
            0.1,
        )

        // Add a spiral staircase
        .add_spiral(
            25.0,   // start radius
            35.0,   // end radius
            15.0,   // height per turn
            3.0,    // number of turns
            24,     // boxes per turn
            Vec3::new(2.0, 1.0, 2.0),  // box size
            |i| {
                let hue = (i as f32 / 72.0) % 1.0;
                hsv_to_rgb(hue, 0.6, 0.8)
            }
        )

        // Add some floating animated boxes
        .add_moving_box(
            Vec3::splat(3.0),
            Vec3::new(0.0, 25.0, 0.0),
            Vec3::new(0.0, 35.0, 0.0),
            [1.0, 0.3, 0.3],
        )
        .add_moving_box(
            Vec3::splat(2.5),
            Vec3::new(10.0, 20.0, 10.0),
            Vec3::new(-10.0, 20.0, -10.0),
            [0.3, 1.0, 0.3],
        )
        .add_moving_box(
            Vec3::splat(2.5),
            Vec3::new(-10.0, 20.0, 10.0),
            Vec3::new(10.0, 20.0, -10.0),
            [0.3, 0.3, 1.0],
        )

        .build()
}
