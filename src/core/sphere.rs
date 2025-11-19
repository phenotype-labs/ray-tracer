use crate::core::bvh::BVHPrimitive;
use crate::math::AABB;
use glam::Vec3;

/// Sphere primitive for ray tracing with material support
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereData {
    pub center: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub material_id: f32,
}

impl SphereData {
    pub fn new(center: Vec3, radius: f32, color: [f32; 3]) -> Self {
        Self {
            center: center.to_array(),
            radius,
            color,
            material_id: -1.0,
        }
    }

    pub fn new_with_material(center: Vec3, radius: f32, color: [f32; 3], material_id: u32) -> Self {
        Self {
            center: center.to_array(),
            radius,
            color,
            material_id: material_id as f32,
        }
    }

    pub fn center(&self) -> Vec3 {
        Vec3::from_array(self.center)
    }

    /// Test ray-sphere intersection using optimized algorithm
    pub fn intersect(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<f32> {
        let oc = ray_origin - self.center();
        let a = ray_dir.dot(ray_dir);
        let half_b = oc.dot(ray_dir);
        let c = oc.dot(oc) - self.radius * self.radius;

        let discriminant = half_b * half_b - a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let t = (-half_b - sqrt_d) / a;

        if t > 1e-4 {
            Some(t)
        } else {
            let t = (-half_b + sqrt_d) / a;
            if t > 1e-4 {
                Some(t)
            } else {
                None
            }
        }
    }

    /// Get normal at point on sphere surface
    pub fn normal_at(&self, point: Vec3) -> Vec3 {
        (point - self.center()).normalize()
    }

    /// Get UV coordinates for point on sphere
    pub fn uv_at(&self, point: Vec3) -> [f32; 2] {
        let normal = self.normal_at(point);
        let u = 0.5 + normal.z.atan2(normal.x) / (2.0 * std::f32::consts::PI);
        let v = 0.5 - normal.y.asin() / std::f32::consts::PI;
        [u, v]
    }
}

impl BVHPrimitive for SphereData {
    fn bounds(&self) -> AABB {
        let center = self.center();
        let radius_vec = Vec3::splat(self.radius);
        AABB::new(center - radius_vec, center + radius_vec)
    }

    fn centroid(&self) -> Vec3 {
        self.center()
    }
}

/// Multi-level sphere container for LOD testing
pub struct MultiLevelSpheres {
    pub spheres: Vec<SphereData>,
    pub lod_levels: Vec<LodLevel>,
}

/// LOD (Level of Detail) level for spheres
#[derive(Clone, Debug)]
pub struct LodLevel {
    pub min_distance: f32,
    pub max_distance: f32,
    pub sphere_indices: Vec<u32>,
}

impl MultiLevelSpheres {
    pub fn new(spheres: Vec<SphereData>) -> Self {
        Self {
            spheres,
            lod_levels: Vec::new(),
        }
    }

    /// Generate LOD levels based on distance thresholds
    pub fn generate_lod_levels(&mut self, distances: &[f32]) {
        self.lod_levels.clear();

        for (i, &max_dist) in distances.iter().enumerate() {
            let min_dist = if i == 0 { 0.0 } else { distances[i - 1] };

            // For simplicity, include all spheres in all levels
            // In practice, you'd filter based on sphere importance/size
            let sphere_indices: Vec<u32> = (0..self.spheres.len() as u32).collect();

            self.lod_levels.push(LodLevel {
                min_distance: min_dist,
                max_distance: max_dist,
                sphere_indices,
            });
        }

        // Add final level for infinite distance
        if let Some(&last_dist) = distances.last() {
            self.lod_levels.push(LodLevel {
                min_distance: last_dist,
                max_distance: f32::INFINITY,
                sphere_indices: (0..self.spheres.len() as u32).collect(),
            });
        }
    }

    /// Get appropriate LOD level for distance
    pub fn get_lod_for_distance(&self, distance: f32) -> Option<&LodLevel> {
        self.lod_levels
            .iter()
            .find(|level| distance >= level.min_distance && distance < level.max_distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_creation() {
        let sphere = SphereData::new(Vec3::new(1.0, 2.0, 3.0), 5.0, [1.0, 0.0, 0.0]);
        assert_eq!(sphere.center(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(sphere.radius, 5.0);
        assert_eq!(sphere.color, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_sphere_bounds() {
        let sphere = SphereData::new(Vec3::ZERO, 2.0, [1.0, 1.0, 1.0]);
        let bounds = sphere.bounds();

        assert_eq!(bounds.min, Vec3::new(-2.0, -2.0, -2.0));
        assert_eq!(bounds.max, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_sphere_intersection_hit() {
        let sphere = SphereData::new(Vec3::new(0.0, 0.0, -5.0), 1.0, [1.0, 0.0, 0.0]);
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);

        let t = sphere.intersect(ray_origin, ray_dir);
        assert!(t.is_some());
        assert!((t.unwrap() - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_sphere_intersection_miss() {
        let sphere = SphereData::new(Vec3::new(0.0, 0.0, -5.0), 1.0, [1.0, 0.0, 0.0]);
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(1.0, 0.0, 0.0); // Ray pointing away

        let t = sphere.intersect(ray_origin, ray_dir);
        assert!(t.is_none());
    }

    #[test]
    fn test_sphere_normal() {
        let sphere = SphereData::new(Vec3::ZERO, 1.0, [1.0, 1.0, 1.0]);
        let point = Vec3::new(1.0, 0.0, 0.0);
        let normal = sphere.normal_at(point);

        assert!((normal - Vec3::new(1.0, 0.0, 0.0)).length() < 0.01);
        assert!((normal.length() - 1.0).abs() < 0.01); // Should be normalized
    }

    #[test]
    fn test_sphere_uv_mapping() {
        let sphere = SphereData::new(Vec3::ZERO, 1.0, [1.0, 1.0, 1.0]);

        // Test point on top of sphere
        let point = Vec3::new(0.0, 1.0, 0.0);
        let uv = sphere.uv_at(point);

        // UV should be roughly in valid range [0, 1]
        assert!(uv[0] >= 0.0 && uv[0] <= 1.0);
        assert!(uv[1] >= 0.0 && uv[1] <= 1.0);
    }

    #[test]
    fn test_multi_level_spheres_creation() {
        let spheres = vec![
            SphereData::new(Vec3::ZERO, 1.0, [1.0, 0.0, 0.0]),
            SphereData::new(Vec3::new(5.0, 0.0, 0.0), 2.0, [0.0, 1.0, 0.0]),
        ];

        let multi = MultiLevelSpheres::new(spheres);
        assert_eq!(multi.spheres.len(), 2);
    }

    #[test]
    fn test_lod_generation() {
        let spheres = vec![
            SphereData::new(Vec3::ZERO, 1.0, [1.0, 0.0, 0.0]),
            SphereData::new(Vec3::new(5.0, 0.0, 0.0), 2.0, [0.0, 1.0, 0.0]),
        ];

        let mut multi = MultiLevelSpheres::new(spheres);
        multi.generate_lod_levels(&[10.0, 50.0, 100.0]);

        assert_eq!(multi.lod_levels.len(), 4); // 3 levels + 1 infinite
    }

    #[test]
    fn test_lod_selection() {
        let spheres = vec![SphereData::new(Vec3::ZERO, 1.0, [1.0, 0.0, 0.0])];

        let mut multi = MultiLevelSpheres::new(spheres);
        multi.generate_lod_levels(&[10.0, 50.0]);

        let lod_close = multi.get_lod_for_distance(5.0);
        assert!(lod_close.is_some());
        assert_eq!(lod_close.unwrap().max_distance, 10.0);

        let lod_far = multi.get_lod_for_distance(75.0);
        assert!(lod_far.is_some());
        assert_eq!(lod_far.unwrap().min_distance, 50.0);
    }

    #[test]
    fn test_sphere_intersection_from_inside() {
        let sphere = SphereData::new(Vec3::ZERO, 5.0, [1.0, 0.0, 0.0]);
        let ray_origin = Vec3::ZERO; // Inside sphere
        let ray_dir = Vec3::new(1.0, 0.0, 0.0);

        let t = sphere.intersect(ray_origin, ray_dir);
        assert!(t.is_some());
        assert!((t.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_sphere_centroid() {
        let sphere = SphereData::new(Vec3::new(1.0, 2.0, 3.0), 1.0, [1.0, 0.0, 0.0]);
        assert_eq!(sphere.centroid(), Vec3::new(1.0, 2.0, 3.0));
    }
}
