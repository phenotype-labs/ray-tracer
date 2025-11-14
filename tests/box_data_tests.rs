use glam::Vec3;
use ray_tracer::types::BoxData;

#[cfg(test)]
mod box_data_tests {
    use super::*;

    #[test]
    fn test_box_data_creation() {
        let min = [0.0, 0.0, 0.0];
        let max = [10.0, 10.0, 10.0];
        let color = [1.0, 0.0, 0.0];

        let box_data = BoxData::new(min, max, color);

        assert_eq!(box_data.min, min);
        assert_eq!(box_data.max, max);
        assert_eq!(box_data.color, color);
        assert_eq!(box_data.is_moving, 0.0, "Static box should have is_moving = 0.0");
    }

    #[test]
    fn test_box_data_center_calculation() {
        let min = [0.0, 0.0, 0.0];
        let max = [10.0, 10.0, 10.0];
        let color = [1.0, 0.0, 0.0];

        let box_data = BoxData::new(min, max, color);

        assert_eq!(box_data.center0, [5.0, 5.0, 5.0]);
        assert_eq!(box_data.center1, [5.0, 5.0, 5.0], "Static box should have same center0 and center1");
    }

    #[test]
    fn test_box_data_half_size_calculation() {
        let min = [0.0, 0.0, 0.0];
        let max = [10.0, 20.0, 30.0];
        let color = [1.0, 0.0, 0.0];

        let box_data = BoxData::new(min, max, color);

        assert_eq!(box_data.half_size, [5.0, 10.0, 15.0]);
    }

    #[test]
    fn test_box_data_offset_box() {
        let min = [5.0, 10.0, 15.0];
        let max = [15.0, 20.0, 25.0];
        let color = [0.0, 1.0, 0.0];

        let box_data = BoxData::new(min, max, color);

        assert_eq!(box_data.center0, [10.0, 15.0, 20.0]);
        assert_eq!(box_data.half_size, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_box_data_bounds() {
        let min = [0.0, 0.0, 0.0];
        let max = [10.0, 10.0, 10.0];
        let color = [1.0, 0.0, 0.0];

        let box_data = BoxData::new(min, max, color);
        let bounds = box_data.bounds();

        assert_eq!(bounds.min, Vec3::from_array(min));
        assert_eq!(bounds.max, Vec3::from_array(max));
    }

    #[test]
    fn test_box_data_is_moving_static() {
        let min = [0.0, 0.0, 0.0];
        let max = [10.0, 10.0, 10.0];
        let color = [1.0, 0.0, 0.0];

        let box_data = BoxData::new(min, max, color);

        assert!(!box_data.is_moving(), "Static box should return false");
    }

    #[test]
    fn test_box_data_moving_creation() {
        let min = [0.0, 0.0, 0.0];
        let max = [10.0, 10.0, 10.0];
        let color = [1.0, 0.0, 0.0];
        let center0 = [5.0, 5.0, 5.0];
        let center1 = [15.0, 15.0, 15.0];
        let half_size = [5.0, 5.0, 5.0];

        let box_data = BoxData::new_moving(min, max, color, center0, center1, half_size);

        assert_eq!(box_data.is_moving, 1.0, "Moving box should have is_moving = 1.0");
        assert_eq!(box_data.center0, center0);
        assert_eq!(box_data.center1, center1);
    }

    #[test]
    fn test_box_data_is_moving_true() {
        let min = [0.0, 0.0, 0.0];
        let max = [20.0, 20.0, 20.0];
        let color = [1.0, 0.0, 0.0];
        let center0 = [5.0, 5.0, 5.0];
        let center1 = [15.0, 15.0, 15.0];
        let half_size = [5.0, 5.0, 5.0];

        let box_data = BoxData::new_moving(min, max, color, center0, center1, half_size);

        assert!(box_data.is_moving(), "Box with different centers should return true");
    }

    #[test]
    fn test_box_data_create_moving_box() {
        let size = Vec3::new(10.0, 10.0, 10.0);
        let center0 = Vec3::new(5.0, 5.0, 5.0);
        let center1 = Vec3::new(15.0, 15.0, 15.0);
        let color = [1.0, 0.0, 0.0];

        let box_data = BoxData::create_moving_box(size, center0, center1, color);

        assert!(box_data.is_moving(), "Should create a moving box");

        // Check that the AABB bounds encompass both positions
        let bounds = box_data.bounds();
        assert!(bounds.min.x <= 0.0, "Min should be less than or equal to first position minus half size");
        assert!(bounds.max.x >= 20.0, "Max should be greater than or equal to second position plus half size");
    }

    #[test]
    fn test_box_data_moving_box_half_size() {
        let size = Vec3::new(10.0, 20.0, 30.0);
        let center0 = Vec3::new(0.0, 0.0, 0.0);
        let center1 = Vec3::new(10.0, 10.0, 10.0);
        let color = [0.0, 1.0, 0.0];

        let box_data = BoxData::create_moving_box(size, center0, center1, color);

        assert_eq!(box_data.half_size, [5.0, 10.0, 15.0]);
    }

    #[test]
    fn test_box_data_negative_coords() {
        let min = [-10.0, -20.0, -30.0];
        let max = [10.0, 20.0, 30.0];
        let color = [0.0, 0.0, 1.0];

        let box_data = BoxData::new(min, max, color);

        assert_eq!(box_data.center0, [0.0, 0.0, 0.0]);
        assert_eq!(box_data.half_size, [10.0, 20.0, 30.0]);
    }
}
