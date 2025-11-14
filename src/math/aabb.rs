use glam::Vec3;

#[derive(Copy, Clone, Debug)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn union(&self, other: &AABB) -> AABB {
        AABB {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_new() {
        let min = Vec3::new(0.0, 0.0, 0.0);
        let max = Vec3::new(1.0, 1.0, 1.0);
        let aabb = AABB::new(min, max);
        assert_eq!(aabb.min, min);
        assert_eq!(aabb.max, max);
    }

    #[test]
    fn test_aabb_center() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 4.0, 6.0));
        let center = aabb.center();
        assert_eq!(center, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_aabb_center_negative() {
        let aabb = AABB::new(Vec3::new(-2.0, -4.0, -6.0), Vec3::new(2.0, 4.0, 6.0));
        let center = aabb.center();
        assert_eq!(center, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_aabb_surface_area_unit_cube() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let area = aabb.surface_area();
        assert!((area - 6.0).abs() < 0.01); // Unit cube has surface area of 6
    }

    #[test]
    fn test_aabb_surface_area_rectangular() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 3.0, 4.0));
        let area = aabb.surface_area();
        // Surface area = 2*(xy + yz + zx) = 2*(2*3 + 3*4 + 4*2) = 2*(6+12+8) = 52
        assert!((area - 52.0).abs() < 0.01);
    }

    #[test]
    fn test_aabb_union_non_overlapping() {
        let aabb1 = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let aabb2 = AABB::new(Vec3::new(2.0, 2.0, 2.0), Vec3::new(3.0, 3.0, 3.0));
        let union = aabb1.union(&aabb2);
        assert_eq!(union.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(union.max, Vec3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn test_aabb_union_overlapping() {
        let aabb1 = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        let aabb2 = AABB::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 3.0));
        let union = aabb1.union(&aabb2);
        assert_eq!(union.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(union.max, Vec3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn test_aabb_union_contained() {
        let aabb1 = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0));
        let aabb2 = AABB::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(2.0, 2.0, 2.0));
        let union = aabb1.union(&aabb2);
        assert_eq!(union.min, aabb1.min);
        assert_eq!(union.max, aabb1.max);
    }

    #[test]
    fn test_aabb_union_negative_coords() {
        let aabb1 = AABB::new(Vec3::new(-3.0, -3.0, -3.0), Vec3::new(-1.0, -1.0, -1.0));
        let aabb2 = AABB::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 3.0));
        let union = aabb1.union(&aabb2);
        assert_eq!(union.min, Vec3::new(-3.0, -3.0, -3.0));
        assert_eq!(union.max, Vec3::new(3.0, 3.0, 3.0));
    }
}
