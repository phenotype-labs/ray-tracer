use glam::Vec3;

pub fn intersect_aabb(ray_origin: Vec3, ray_dir: Vec3, box_min: Vec3, box_max: Vec3) -> f32 {
    const EPSILON: f32 = 1e-8;

    // Precompute inverse direction with epsilon clamping to avoid division by zero
    // When ray component is near zero, clamp to large value (effectively infinity)
    let inv_dir = Vec3::new(
        if ray_dir.x.abs() < EPSILON { 1.0 / EPSILON.copysign(ray_dir.x) } else { 1.0 / ray_dir.x },
        if ray_dir.y.abs() < EPSILON { 1.0 / EPSILON.copysign(ray_dir.y) } else { 1.0 / ray_dir.y },
        if ray_dir.z.abs() < EPSILON { 1.0 / EPSILON.copysign(ray_dir.z) } else { 1.0 / ray_dir.z },
    );

    let t_min = (box_min - ray_origin) * inv_dir;
    let t_max = (box_max - ray_origin) * inv_dir;

    let t1 = t_min.min(t_max);
    let t2 = t_min.max(t_max);

    let t_near = t1.x.max(t1.y).max(t1.z);
    let t_far = t2.x.min(t2.y).min(t2.z);

    if t_near > t_far || t_far < 0.0 {
        return -1.0;
    }

    if t_near < 0.0 {
        if t_far > 0.001 {
            t_far
        } else {
            -1.0
        }
    } else {
        t_near
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersect_aabb_hit() {
        let ray_origin = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0);
        let box_min = Vec3::new(5.0, -1.0, -1.0);
        let box_max = Vec3::new(10.0, 1.0, 1.0);
        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);
        assert!(t > 0.0);
        assert!((t - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_intersect_aabb_miss() {
        let ray_origin = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0);
        let box_min = Vec3::new(5.0, 2.0, 2.0);
        let box_max = Vec3::new(10.0, 3.0, 3.0);
        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);
        assert!(t < 0.0);
    }

    #[test]
    fn test_intersect_aabb_inside() {
        let ray_origin = Vec3::new(5.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0);
        let box_min = Vec3::new(0.0, -1.0, -1.0);
        let box_max = Vec3::new(10.0, 1.0, 1.0);
        let t = intersect_aabb(ray_origin, ray_dir, box_min, box_max);
        assert!(t > 0.0);
    }
}
