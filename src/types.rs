use glam::Vec3;

/// Camera uniform buffer data for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub position: [f32; 3],
    pub _pad1: f32,
    pub forward: [f32; 3],
    pub _pad2: f32,
    pub right: [f32; 3],
    pub _pad3: f32,
    pub up: [f32; 3],
    pub time: f32, // Animation time for moving objects
}

/// Box primitive data for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoxData {
    pub min: [f32; 3],
    pub is_moving: f32, // 1.0 if moving, 0.0 if static
    pub max: [f32; 3],
    pub _pad2: f32,
    pub color: [f32; 3],
    pub _pad3: f32,
    pub center0: [f32; 3], // Start position for moving objects
    pub _pad4: f32,
    pub center1: [f32; 3], // End position for moving objects
    pub _pad5: f32,
    pub half_size: [f32; 3], // Actual box half-dimensions
    pub _pad6: f32,
}

impl BoxData {
    pub const fn new(min: [f32; 3], max: [f32; 3], color: [f32; 3]) -> Self {
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let half_size = [
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        ];
        Self {
            min,
            is_moving: 0.0, // Static object
            max,
            _pad2: 0.0,
            color,
            _pad3: 0.0,
            center0: center,
            _pad4: 0.0,
            center1: center, // Same as center0 for static objects
            _pad5: 0.0,
            half_size,
            _pad6: 0.0,
        }
    }

    pub fn new_moving(min: [f32; 3], max: [f32; 3], color: [f32; 3], center0: [f32; 3], center1: [f32; 3], half_size: [f32; 3]) -> Self {
        Self {
            min,
            is_moving: 1.0, // Moving object
            max,
            _pad2: 0.0,
            color,
            _pad3: 0.0,
            center0,
            _pad4: 0.0,
            center1,
            _pad5: 0.0,
            half_size,
            _pad6: 0.0,
        }
    }

    pub fn bounds(&self) -> AABB {
        AABB {
            min: Vec3::from_array(self.min),
            max: Vec3::from_array(self.max),
        }
    }

    pub fn is_moving(&self) -> bool {
        let c0 = Vec3::from_array(self.center0);
        let c1 = Vec3::from_array(self.center1);
        c0.distance(c1) > 0.001
    }

    /// Create a moving box with proper AABB that encompasses the entire motion
    pub fn create_moving_box(
        size: Vec3,
        center0: Vec3,
        center1: Vec3,
        color: [f32; 3],
    ) -> Self {
        let half_size = size * 0.5;

        // Calculate AABB that encompasses both positions
        let min0 = center0 - half_size;
        let max0 = center0 + half_size;
        let min1 = center1 - half_size;
        let max1 = center1 + half_size;

        let aabb_min = min0.min(min1);
        let aabb_max = max0.max(max1);

        // Add extra padding to ensure grid cells contain the box at all positions
        let padding = Vec3::splat(0.5);
        let padded_min = aabb_min - padding;
        let padded_max = aabb_max + padding;

        Self::new_moving(
            padded_min.to_array(),
            padded_max.to_array(),
            color,
            center0.to_array(),
            center1.to_array(),
            half_size.to_array(),
        )
    }
}

/// Axis-Aligned Bounding Box
#[derive(Copy, Clone, Debug)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
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
