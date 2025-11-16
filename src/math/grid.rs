use glam::Vec3;

pub fn world_to_cell(pos: Vec3, bounds_min: Vec3, cell_size: f32) -> (i32, i32, i32) {
    debug_assert!(cell_size > 0.0, "cell_size must be positive");
    debug_assert!(pos.is_finite() && bounds_min.is_finite(), "inputs must be finite");

    let rel_pos = pos - bounds_min;
    // Use multiplication by inverse to avoid division issues
    let inv_cell_size = 1.0 / cell_size;
    (
        (rel_pos.x * inv_cell_size).floor() as i32,
        (rel_pos.y * inv_cell_size).floor() as i32,
        (rel_pos.z * inv_cell_size).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_to_cell_origin() {
        let pos = Vec3::new(0.0, 0.0, 0.0);
        let bounds_min = Vec3::new(0.0, 0.0, 0.0);
        let cell_size = 16.0;
        let cell = world_to_cell(pos, bounds_min, cell_size);
        assert_eq!(cell, (0, 0, 0));
    }

    #[test]
    fn test_world_to_cell_offset() {
        let pos = Vec3::new(20.0, 30.0, 40.0);
        let bounds_min = Vec3::new(0.0, 0.0, 0.0);
        let cell_size = 10.0;
        let cell = world_to_cell(pos, bounds_min, cell_size);
        assert_eq!(cell, (2, 3, 4));
    }

    #[test]
    fn test_world_to_cell_negative() {
        let pos = Vec3::new(-5.0, -5.0, -5.0);
        let bounds_min = Vec3::new(-10.0, -10.0, -10.0);
        let cell_size = 10.0;
        let cell = world_to_cell(pos, bounds_min, cell_size);
        assert_eq!(cell, (0, 0, 0));
    }
}
