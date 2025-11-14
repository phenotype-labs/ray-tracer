use glam::Vec3;

/// Convert world position to grid cell coordinates
fn world_to_cell(pos: Vec3, bounds_min: Vec3, cell_size: f32) -> (i32, i32, i32) {
    let rel_pos = pos - bounds_min;
    (
        (rel_pos.x / cell_size).floor() as i32,
        (rel_pos.y / cell_size).floor() as i32,
        (rel_pos.z / cell_size).floor() as i32,
    )
}

/// Ray-AABB intersection test using slab method
fn intersect_aabb(ray_origin: Vec3, ray_dir: Vec3, box_min: Vec3, box_max: Vec3) -> f32 {
    let t_min = (box_min - ray_origin) / ray_dir;
    let t_max = (box_max - ray_origin) / ray_dir;

    let t1 = t_min.min(t_max);
    let t2 = t_min.max(t_max);

    let t_near = t1.x.max(t1.y).max(t1.z);
    let t_far = t2.x.min(t2.y).min(t2.z);

    if t_near > t_far || t_far < 0.0 {
        return -1.0;
    }

    if t_near < 0.0 {
        t_far
    } else {
        t_near
    }
}

#[cfg(test)]
mod ray_intersection_tests {
    use super::*;

    #[test]
    fn test_ray_hits_aabb_from_outside() {
        let ray_origin = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0).normalize();
        let box_min = Vec3::new(5.0, -1.0, -1.0);
        let box_max = Vec3::new(10.0, 1.0, 1.0);

        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);

        assert!(t > 0.0, "Ray should hit AABB");
        assert!((t - 5.0).abs() < 0.001, "Hit distance should be ~5.0, got {}", t);
    }

    #[test]
    fn test_ray_misses_aabb() {
        let ray_origin = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0).normalize();
        let box_min = Vec3::new(5.0, 5.0, 5.0);
        let box_max = Vec3::new(10.0, 10.0, 10.0);

        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);

        assert_eq!(t, -1.0, "Ray should miss AABB");
    }

    #[test]
    fn test_ray_starts_inside_aabb() {
        let ray_origin = Vec3::new(5.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0).normalize();
        let box_min = Vec3::new(0.0, -1.0, -1.0);
        let box_max = Vec3::new(10.0, 1.0, 1.0);

        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);

        assert!(t > 0.0, "Should return exit distance when ray starts inside");
    }

    #[test]
    fn test_ray_hits_aabb_at_angle() {
        let ray_origin = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 1.0, 1.0).normalize();
        let box_min = Vec3::new(5.0, 5.0, 5.0);
        let box_max = Vec3::new(10.0, 10.0, 10.0);

        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);

        assert!(t > 0.0, "Ray should hit AABB at angle");

        let hit_point = ray_origin + ray_dir * t;
        assert!(
            hit_point.x >= box_min.x - 0.001 && hit_point.x <= box_max.x + 0.001,
            "Hit point x should be within AABB bounds"
        );
        assert!(
            hit_point.y >= box_min.y - 0.001 && hit_point.y <= box_max.y + 0.001,
            "Hit point y should be within AABB bounds"
        );
        assert!(
            hit_point.z >= box_min.z - 0.001 && hit_point.z <= box_max.z + 0.001,
            "Hit point z should be within AABB bounds"
        );
    }

    #[test]
    fn test_ray_pointing_away_from_aabb() {
        let ray_origin = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(-1.0, 0.0, 0.0).normalize();
        let box_min = Vec3::new(5.0, -1.0, -1.0);
        let box_max = Vec3::new(10.0, 1.0, 1.0);

        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);

        assert_eq!(t, -1.0, "Ray pointing away should not hit AABB");
    }

    #[test]
    fn test_ray_parallel_to_aabb_face() {
        let ray_origin = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0).normalize();
        let box_min = Vec3::new(5.0, 1.0, -1.0);
        let box_max = Vec3::new(10.0, 2.0, 1.0);

        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);

        assert_eq!(t, -1.0, "Ray parallel to and outside AABB should miss");
    }
}

#[cfg(test)]
mod world_to_cell_tests {
    use super::*;

    #[test]
    fn test_world_to_cell_origin() {
        let pos = Vec3::new(0.0, 0.0, 0.0);
        let bounds_min = Vec3::new(0.0, 0.0, 0.0);
        let cell_size = 1.0;

        let cell = world_to_cell(pos, bounds_min, cell_size);

        assert_eq!(cell, (0, 0, 0));
    }

    #[test]
    fn test_world_to_cell_positive_offset() {
        let pos = Vec3::new(5.5, 10.7, 15.2);
        let bounds_min = Vec3::new(0.0, 0.0, 0.0);
        let cell_size = 5.0;

        let cell = world_to_cell(pos, bounds_min, cell_size);

        assert_eq!(cell, (1, 2, 3));
    }

    #[test]
    fn test_world_to_cell_negative_bounds() {
        let pos = Vec3::new(-5.0, -10.0, -15.0);
        let bounds_min = Vec3::new(-20.0, -20.0, -20.0);
        let cell_size = 10.0;

        let cell = world_to_cell(pos, bounds_min, cell_size);

        assert_eq!(cell, (1, 1, 0));
    }

    #[test]
    fn test_world_to_cell_cell_boundary() {
        let pos = Vec3::new(10.0, 20.0, 30.0);
        let bounds_min = Vec3::new(0.0, 0.0, 0.0);
        let cell_size = 10.0;

        let cell = world_to_cell(pos, bounds_min, cell_size);

        assert_eq!(cell, (1, 2, 3), "Position exactly on boundary should map to next cell");
    }

    #[test]
    fn test_world_to_cell_fractional_cell_size() {
        let pos = Vec3::new(1.5, 3.2, 4.8);
        let bounds_min = Vec3::new(0.0, 0.0, 0.0);
        let cell_size = 0.5;

        let cell = world_to_cell(pos, bounds_min, cell_size);

        assert_eq!(cell, (3, 6, 9));
    }

    #[test]
    fn test_world_to_cell_with_grid_offset() {
        let pos = Vec3::new(-28.62, 28.00, -4.03);
        let bounds_min = Vec3::new(-201.0, -2.0, -201.0);
        let cell_size = 16.0;

        let cell = world_to_cell(pos, bounds_min, cell_size);

        assert!(cell.0 >= 0 && cell.1 >= 0 && cell.2 >= 0,
                "Grid cells should be non-negative, got {:?}", cell);
    }
}

#[cfg(test)]
mod ray_direction_tests {
    use super::*;

    #[test]
    fn test_ray_direction_normalization() {
        let dir = Vec3::new(3.0, 4.0, 0.0);
        let normalized = dir.normalize();

        let length = normalized.length();
        assert!((length - 1.0).abs() < 0.0001, "Normalized ray direction should have length 1.0");
    }

    #[test]
    fn test_ray_direction_preserves_direction() {
        let dir = Vec3::new(1.0, 1.0, 1.0);
        let normalized = dir.normalize();

        assert!(normalized.x > 0.0 && normalized.y > 0.0 && normalized.z > 0.0,
                "Normalization should preserve direction signs");
    }

    #[test]
    fn test_opposite_ray_directions() {
        let dir1 = Vec3::new(1.0, 0.0, 0.0).normalize();
        let dir2 = Vec3::new(-1.0, 0.0, 0.0).normalize();

        let dot = dir1.dot(dir2);
        assert!((dot + 1.0).abs() < 0.0001, "Opposite directions should have dot product of -1.0");
    }

    #[test]
    fn test_perpendicular_ray_directions() {
        let dir1 = Vec3::new(1.0, 0.0, 0.0).normalize();
        let dir2 = Vec3::new(0.0, 1.0, 0.0).normalize();

        let dot = dir1.dot(dir2);
        assert!(dot.abs() < 0.0001, "Perpendicular directions should have dot product of 0.0");
    }
}

#[cfg(test)]
mod dda_traversal_tests {
    use super::*;

    #[test]
    fn test_ray_step_direction_positive() {
        let direction = Vec3::new(1.0, 1.0, 1.0);

        let step = (
            if direction.x >= 0.0 { 1 } else { -1 },
            if direction.y >= 0.0 { 1 } else { -1 },
            if direction.z >= 0.0 { 1 } else { -1 },
        );

        assert_eq!(step, (1, 1, 1));
    }

    #[test]
    fn test_ray_step_direction_negative() {
        let direction = Vec3::new(-1.0, -1.0, -1.0);

        let step = (
            if direction.x >= 0.0 { 1 } else { -1 },
            if direction.y >= 0.0 { 1 } else { -1 },
            if direction.z >= 0.0 { 1 } else { -1 },
        );

        assert_eq!(step, (-1, -1, -1));
    }

    #[test]
    fn test_ray_step_direction_mixed() {
        let direction = Vec3::new(1.0, -1.0, 1.0);

        let step = (
            if direction.x >= 0.0 { 1 } else { -1 },
            if direction.y >= 0.0 { 1 } else { -1 },
            if direction.z >= 0.0 { 1 } else { -1 },
        );

        assert_eq!(step, (1, -1, 1));
    }

    #[test]
    fn test_t_delta_calculation() {
        let direction = Vec3::new(1.0, 0.5, 0.25).normalize();
        let cell_size = 16.0;

        let t_delta = Vec3::new(
            (cell_size / direction.x).abs(),
            (cell_size / direction.y).abs(),
            (cell_size / direction.z).abs(),
        );

        assert!(t_delta.x > 0.0, "t_delta x should be positive");
        assert!(t_delta.y > 0.0, "t_delta y should be positive");
        assert!(t_delta.z > 0.0, "t_delta z should be positive");
        assert!(t_delta.x < t_delta.y, "Larger direction component should have smaller t_delta");
        assert!(t_delta.y < t_delta.z, "Larger direction component should have smaller t_delta");
    }
}
