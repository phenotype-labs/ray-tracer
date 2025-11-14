use glam::Vec3;
use ray_tracer::types::AABB;

#[cfg(test)]
mod aabb_tests {
    use super::*;

    #[test]
    fn test_aabb_union_creates_bounding_box() {
        let aabb1 = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(10.0, 10.0, 10.0),
        };
        let aabb2 = AABB {
            min: Vec3::new(5.0, 5.0, 5.0),
            max: Vec3::new(15.0, 15.0, 15.0),
        };

        let union = aabb1.union(&aabb2);

        assert_eq!(union.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(union.max, Vec3::new(15.0, 15.0, 15.0));
    }

    #[test]
    fn test_aabb_union_with_negative_coords() {
        let aabb1 = AABB {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(0.0, 0.0, 0.0),
        };
        let aabb2 = AABB {
            min: Vec3::new(-5.0, -5.0, -5.0),
            max: Vec3::new(5.0, 5.0, 5.0),
        };

        let union = aabb1.union(&aabb2);

        assert_eq!(union.min, Vec3::new(-10.0, -10.0, -10.0));
        assert_eq!(union.max, Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_aabb_union_with_contained_box() {
        let aabb1 = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(10.0, 10.0, 10.0),
        };
        let aabb2 = AABB {
            min: Vec3::new(2.0, 2.0, 2.0),
            max: Vec3::new(8.0, 8.0, 8.0),
        };

        let union = aabb1.union(&aabb2);

        assert_eq!(union.min, aabb1.min, "Union should equal larger box");
        assert_eq!(union.max, aabb1.max, "Union should equal larger box");
    }

    #[test]
    fn test_aabb_center_calculation() {
        let aabb = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(10.0, 10.0, 10.0),
        };

        let center = aabb.center();

        assert_eq!(center, Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_aabb_center_with_negative_coords() {
        let aabb = AABB {
            min: Vec3::new(-10.0, -20.0, -30.0),
            max: Vec3::new(10.0, 20.0, 30.0),
        };

        let center = aabb.center();

        assert_eq!(center, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_aabb_center_offset_box() {
        let aabb = AABB {
            min: Vec3::new(5.0, 10.0, 15.0),
            max: Vec3::new(15.0, 20.0, 25.0),
        };

        let center = aabb.center();

        assert_eq!(center, Vec3::new(10.0, 15.0, 20.0));
    }

    #[test]
    fn test_aabb_surface_area_unit_cube() {
        let aabb = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(1.0, 1.0, 1.0),
        };

        let surface_area = aabb.surface_area();

        assert_eq!(surface_area, 6.0, "Unit cube should have surface area of 6.0");
    }

    #[test]
    fn test_aabb_surface_area_rectangular_box() {
        let aabb = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(2.0, 3.0, 4.0),
        };

        let surface_area = aabb.surface_area();

        // Surface area = 2 * (width*height + height*depth + depth*width)
        // = 2 * (2*3 + 3*4 + 4*2) = 2 * (6 + 12 + 8) = 2 * 26 = 52
        assert_eq!(surface_area, 52.0);
    }

    #[test]
    fn test_aabb_surface_area_with_offset() {
        let aabb = AABB {
            min: Vec3::new(5.0, 5.0, 5.0),
            max: Vec3::new(6.0, 6.0, 6.0),
        };

        let surface_area = aabb.surface_area();

        assert_eq!(surface_area, 6.0, "Offset unit cube should still have surface area of 6.0");
    }

    #[test]
    fn test_aabb_degenerate_flat_box() {
        let aabb = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(10.0, 10.0, 0.0),
        };

        let surface_area = aabb.surface_area();

        // Flat box in XY plane: 2 * (10*10 + 10*0 + 0*10) = 2 * 100 = 200
        assert_eq!(surface_area, 200.0);
    }
}
