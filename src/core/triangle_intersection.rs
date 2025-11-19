use crate::types::TriangleData;
use glam::Vec3;

/// Result of triangle intersection test
#[derive(Debug, Clone, Copy)]
pub struct TriangleIntersection {
    pub t: f32,           // Distance along ray
    pub u: f32,           // Barycentric coordinate u
    pub v: f32,           // Barycentric coordinate v
    pub normal: Vec3,     // Surface normal at intersection
}

impl TriangleIntersection {
    /// Get barycentric coordinates (u, v, w) where w = 1 - u - v
    pub fn barycentric(&self) -> (f32, f32, f32) {
        (self.u, self.v, 1.0 - self.u - self.v)
    }

    /// Interpolate UV coordinates using barycentric coordinates
    pub fn interpolate_uv(&self, uv0: [f32; 2], uv1: [f32; 2], uv2: [f32; 2]) -> [f32; 2] {
        let (u, v, w) = self.barycentric();
        [
            w * uv0[0] + u * uv1[0] + v * uv2[0],
            w * uv0[1] + u * uv1[1] + v * uv2[1],
        ]
    }
}

/// Möller-Trumbore ray-triangle intersection algorithm
/// Fast, branch-free algorithm for ray-triangle intersection
pub fn moller_trumbore_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<TriangleIntersection> {
    const EPSILON: f32 = 1e-6;

    // Find vectors for two edges sharing v0
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;

    // Calculate determinant
    let h = ray_dir.cross(edge2);
    let a = edge1.dot(h);

    // Ray is parallel to triangle
    if a.abs() < EPSILON {
        return None;
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * s.dot(h);

    // Intersection outside triangle
    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray_dir.dot(q);

    // Intersection outside triangle
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    // Calculate t to find intersection point
    let t = f * edge2.dot(q);

    // Ray intersection behind origin
    if t < EPSILON {
        return None;
    }

    // Calculate normal (counter-clockwise winding)
    let normal = edge1.cross(edge2).normalize();

    Some(TriangleIntersection { t, u, v, normal })
}

/// Optimized triangle intersection for TriangleData
pub fn intersect_triangle_data(
    ray_origin: Vec3,
    ray_dir: Vec3,
    triangle: &TriangleData,
) -> Option<TriangleIntersection> {
    moller_trumbore_intersect(
        ray_origin,
        ray_dir,
        Vec3::from_array(triangle.v0),
        Vec3::from_array(triangle.v1),
        Vec3::from_array(triangle.v2),
    )
}

/// Watertight ray-triangle intersection (Woop et al. 2013)
/// More robust for edge cases, slightly slower
pub fn watertight_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<TriangleIntersection> {
    const EPSILON: f32 = 1e-6;

    // Translate vertices based on ray origin
    let a = v0 - ray_origin;
    let b = v1 - ray_origin;
    let c = v2 - ray_origin;

    // Determine major axis for projection
    let abs_dir = ray_dir.abs();
    let kz = if abs_dir.x > abs_dir.y && abs_dir.x > abs_dir.z {
        0
    } else if abs_dir.y > abs_dir.z {
        1
    } else {
        2
    };
    let kx = (kz + 1) % 3;
    let ky = (kx + 1) % 3;

    // Swap dimensions to align ray direction with +z axis
    let d = Vec3::new(ray_dir[kx], ray_dir[ky], ray_dir[kz]);

    // Shear constants
    let sx = d.x / d.z;
    let sy = d.y / d.z;
    let sz = 1.0 / d.z;

    // Calculate sheared vertices
    let ax = a[kx] - sx * a[kz];
    let ay = a[ky] - sy * a[kz];
    let bx = b[kx] - sx * b[kz];
    let by = b[ky] - sy * b[kz];
    let cx = c[kx] - sx * c[kz];
    let cy = c[ky] - sy * c[kz];

    // Calculate scaled barycentric coordinates
    let u = cx * by - cy * bx;
    let v = ax * cy - ay * cx;
    let w = bx * ay - by * ax;

    // Perform edge tests
    if (u < 0.0 || v < 0.0 || w < 0.0) && (u > 0.0 || v > 0.0 || w > 0.0) {
        return None;
    }

    // Calculate determinant
    let det = u + v + w;
    if det.abs() < EPSILON {
        return None;
    }

    // Calculate scaled z-coordinates of vertices
    let az = sz * a[kz];
    let bz = sz * b[kz];
    let cz = sz * c[kz];
    let t = u * az + v * bz + w * cz;

    // Normalize
    let inv_det = 1.0 / det;
    let t = t * inv_det;
    let u = u * inv_det;
    let v = v * inv_det;

    if t < EPSILON {
        return None;
    }

    // Calculate normal
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let normal = edge1.cross(edge2).normalize();

    Some(TriangleIntersection { t, u, v, normal })
}

/// Batch intersection test for multiple triangles
pub fn batch_intersect_triangles(
    ray_origin: Vec3,
    ray_dir: Vec3,
    triangles: &[TriangleData],
    indices: &[u32],
) -> Option<(usize, TriangleIntersection)> {
    let mut closest_hit = None;
    let mut closest_t = f32::INFINITY;

    for &idx in indices {
        let triangle = &triangles[idx as usize];
        if let Some(hit) = intersect_triangle_data(ray_origin, ray_dir, triangle) {
            if hit.t < closest_t {
                closest_t = hit.t;
                closest_hit = Some((idx as usize, hit));
            }
        }
    }

    closest_hit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_triangle() -> (Vec3, Vec3, Vec3) {
        (
            Vec3::new(-1.0, 0.0, -5.0),
            Vec3::new(1.0, 0.0, -5.0),
            Vec3::new(0.0, 1.0, -5.0),
        )
    }

    #[test]
    fn test_moller_trumbore_hit() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0); // Shoot straight down Z axis

        let hit = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        assert!(hit.is_some());

        let hit = hit.unwrap();
        assert!(hit.t > 0.0);
        assert!(hit.u >= 0.0 && hit.u <= 1.0);
        assert!(hit.v >= 0.0 && hit.v <= 1.0);
        assert!(hit.u + hit.v <= 1.0);
    }

    #[test]
    fn test_moller_trumbore_miss() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(5.0, 0.0, -1.0).normalize(); // Ray misses triangle

        let hit = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        assert!(hit.is_none());
    }

    #[test]
    fn test_behind_ray() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, 1.0); // Ray pointing away

        let hit = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        assert!(hit.is_none());
    }

    #[test]
    fn test_barycentric_coordinates() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);

        let hit = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        assert!(hit.is_some());

        let (u, v, w) = hit.unwrap().barycentric();
        assert!((u + v + w - 1.0).abs() < 1e-5); // Should sum to 1
    }

    #[test]
    fn test_uv_interpolation() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);

        let hit = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        assert!(hit.is_some());

        let uv0 = [0.0, 0.0];
        let uv1 = [1.0, 0.0];
        let uv2 = [0.5, 1.0];

        let interpolated = hit.unwrap().interpolate_uv(uv0, uv1, uv2);
        assert!(interpolated[0] >= 0.0 && interpolated[0] <= 1.0);
        assert!(interpolated[1] >= 0.0 && interpolated[1] <= 1.0);
    }

    #[test]
    fn test_watertight_vs_moller_trumbore() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);

        let hit1 = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        let hit2 = watertight_intersect(ray_origin, ray_dir, v0, v1, v2);

        assert!(hit1.is_some());
        assert!(hit2.is_some());

        // Both should give similar results
        let h1 = hit1.unwrap();
        let h2 = hit2.unwrap();
        assert!((h1.t - h2.t).abs() < 0.1);
    }

    #[test]
    fn test_triangle_data_intersect() {
        let triangle = TriangleData::new(
            [-1.0, 0.0, -5.0],
            [1.0, 0.0, -5.0],
            [0.0, 1.0, -5.0],
            [0.0, 0.0],
            [1.0, 0.0],
            [0.5, 1.0],
            0,
        );

        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);

        let hit = intersect_triangle_data(ray_origin, ray_dir, &triangle);
        assert!(hit.is_some());
    }

    #[test]
    fn test_batch_intersection() {
        let triangles = vec![
            TriangleData::new(
                [-1.0, 0.0, -10.0],
                [1.0, 0.0, -10.0],
                [0.0, 1.0, -10.0],
                [0.0, 0.0],
                [1.0, 0.0],
                [0.5, 1.0],
                0,
            ),
            TriangleData::new(
                [-1.0, 0.0, -5.0],
                [1.0, 0.0, -5.0],
                [0.0, 1.0, -5.0],
                [0.0, 0.0],
                [1.0, 0.0],
                [0.5, 1.0],
                0,
            ),
        ];

        let indices = vec![0, 1];
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);

        let hit = batch_intersect_triangles(ray_origin, ray_dir, &triangles, &indices);
        assert!(hit.is_some());

        let (idx, _) = hit.unwrap();
        assert_eq!(idx, 1); // Should hit closer triangle (index 1)
    }

    #[test]
    fn test_edge_case_parallel_ray() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::new(0.0, 0.0, -5.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0); // Parallel to triangle plane

        let hit = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        assert!(hit.is_none());
    }

    #[test]
    fn test_normal_calculation() {
        let (v0, v1, v2) = create_test_triangle();
        let ray_origin = Vec3::ZERO;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);

        let hit = moller_trumbore_intersect(ray_origin, ray_dir, v0, v1, v2);
        assert!(hit.is_some());

        let normal = hit.unwrap().normal;
        assert!((normal.length() - 1.0).abs() < 1e-5); // Normal should be normalized
        assert!(normal.z > 0.0); // Should point towards camera
    }
}
