use glam::Vec3;
use crate::math::AABB;

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
    pub time: f32,
    pub lod_factor: f32,
    pub min_pixel_size: f32,
    pub show_grid: f32,
    pub _pad4: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoxData {
    pub min: [f32; 3],
    pub is_moving: f32,
    pub max: [f32; 3],
    pub _pad2: f32,
    pub color: [f32; 3],
    pub reflectivity: f32,
    pub center0: [f32; 3],
    pub _pad4: f32,
    pub center1: [f32; 3],
    pub _pad5: f32,
    pub half_size: [f32; 3],
    pub _pad6: f32,
}

impl BoxData {
    const fn calculate_center(min: [f32; 3], max: [f32; 3]) -> [f32; 3] {
        [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ]
    }

    const fn calculate_half_size(min: [f32; 3], max: [f32; 3]) -> [f32; 3] {
        [
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        ]
    }

    pub const fn new(min: [f32; 3], max: [f32; 3], color: [f32; 3]) -> Self {
        let center = Self::calculate_center(min, max);
        let half_size = Self::calculate_half_size(min, max);
        Self {
            min,
            is_moving: 0.0,
            max,
            _pad2: 0.0,
            color,
            reflectivity: 0.0,
            center0: center,
            _pad4: 0.0,
            center1: center,
            _pad5: 0.0,
            half_size,
            _pad6: 0.0,
        }
    }

    pub const fn new_reflective(min: [f32; 3], max: [f32; 3], color: [f32; 3], reflectivity: f32) -> Self {
        let center = Self::calculate_center(min, max);
        let half_size = Self::calculate_half_size(min, max);
        Self {
            min,
            is_moving: 0.0,
            max,
            _pad2: 0.0,
            color,
            reflectivity,
            center0: center,
            _pad4: 0.0,
            center1: center,
            _pad5: 0.0,
            half_size,
            _pad6: 0.0,
        }
    }

    pub fn new_moving(min: [f32; 3], max: [f32; 3], color: [f32; 3], center0: [f32; 3], center1: [f32; 3], half_size: [f32; 3]) -> Self {
        Self {
            min,
            is_moving: 1.0,
            max,
            _pad2: 0.0,
            color,
            reflectivity: 0.0,
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

    pub fn create_moving_box(
        size: Vec3,
        center0: Vec3,
        center1: Vec3,
        color: [f32; 3],
    ) -> Self {
        let half_size = size * 0.5;

        let min0 = center0 - half_size;
        let max0 = center0 + half_size;
        let min1 = center1 - half_size;
        let max1 = center1 + half_size;

        let aabb_min = min0.min(min1);
        let aabb_max = max0.max(max1);

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


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RayDebugInfo {
    pub ray_origin: [f32; 3],
    pub hit: f32,
    pub ray_direction: [f32; 3],
    pub distance: f32,
    pub hit_position: [f32; 3],
    pub object_id: f32,
    pub hit_normal: [f32; 3],
    pub num_steps: f32,
    pub hit_color: [f32; 3],
    pub _pad: f32,
}

impl Default for RayDebugInfo {
    fn default() -> Self {
        Self {
            ray_origin: [0.0; 3],
            hit: 0.0,
            ray_direction: [0.0; 3],
            distance: 0.0,
            hit_position: [0.0; 3],
            object_id: -1.0,
            hit_normal: [0.0; 3],
            num_steps: 0.0,
            hit_color: [0.0; 3],
            _pad: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DebugParams {
    pub debug_pixel: [u32; 2],
    pub enabled: u32,
    pub _pad: u32,
}

/// Triangle data for ray tracing with UV coordinates
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TriangleData {
    pub v0: [f32; 3],
    pub material_id: f32,
    pub v1: [f32; 3],
    pub _pad1: f32,
    pub v2: [f32; 3],
    pub _pad2: f32,
    pub uv0: [f32; 2],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub _pad3: [f32; 2],
}

impl TriangleData {
    pub fn new(
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        uv0: [f32; 2],
        uv1: [f32; 2],
        uv2: [f32; 2],
        material_id: u32,
    ) -> Self {
        Self {
            v0,
            material_id: material_id as f32,
            v1,
            _pad1: 0.0,
            v2,
            _pad2: 0.0,
            uv0,
            uv1,
            uv2,
            _pad3: [0.0, 0.0],
        }
    }

    pub fn bounds(&self) -> AABB {
        let v0 = Vec3::from_array(self.v0);
        let v1 = Vec3::from_array(self.v1);
        let v2 = Vec3::from_array(self.v2);

        let min = v0.min(v1).min(v2);
        let max = v0.max(v1).max(v2);

        AABB { min, max }
    }
}

/// Material data for textures and colors
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialData {
    pub base_color: [f32; 4],
    pub texture_index: i32,  // -1 means no texture
    pub metallic: f32,
    pub roughness: f32,
    pub _pad: f32,
}

impl MaterialData {
    pub fn new_color(color: [f32; 4]) -> Self {
        Self {
            base_color: color,
            texture_index: -1,
            metallic: 0.0,
            roughness: 1.0,
            _pad: 0.0,
        }
    }

    pub fn new_textured(color: [f32; 4], texture_index: u32) -> Self {
        Self {
            base_color: color,
            texture_index: texture_index as i32,
            metallic: 0.0,
            roughness: 1.0,
            _pad: 0.0,
        }
    }
}
