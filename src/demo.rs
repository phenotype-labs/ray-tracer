use glam::Vec3;
use crate::types::BoxData;
use crate::math::hsv_to_rgb;

/// Demo module provides reusable primitives and builders for creating ray tracer scenes
///
/// # Examples
///
/// ```
/// use ray_tracer::demo::*;
///
/// let scene = DemoBuilder::new()
///     .add_ground([0.2, 0.2, 0.2])
///     .add_ring(20.0, 32, 10.0, rainbow_gradient(32))
///     .build();
/// ```

// ============================================================================
// Primitive Generators - Create common geometric patterns
// ============================================================================

/// Creates a ground plane
pub fn ground(min: [f32; 3], max: [f32; 3], color: [f32; 3]) -> BoxData {
    BoxData::new(min, max, color)
}

/// Creates a reflective ground plane
pub fn reflective_ground(min: [f32; 3], max: [f32; 3], color: [f32; 3], reflectivity: f32) -> BoxData {
    BoxData::new_reflective(min, max, color, reflectivity)
}

/// Creates a single box at position with size
pub fn box_at(position: Vec3, size: Vec3, color: [f32; 3]) -> BoxData {
    BoxData::new(
        (position - size * 0.5).to_array(),
        (position + size * 0.5).to_array(),
        color,
    )
}

/// Creates a reflective box at position with size
pub fn reflective_box_at(position: Vec3, size: Vec3, color: [f32; 3], reflectivity: f32) -> BoxData {
    BoxData::new_reflective(
        (position - size * 0.5).to_array(),
        (position + size * 0.5).to_array(),
        color,
        reflectivity,
    )
}

/// Creates a grid of boxes in the XZ plane
pub fn grid<F>(
    center: Vec3,
    box_size: f32,
    spacing: f32,
    count_x: usize,
    count_z: usize,
    height: f32,
    colors: F,
) -> Vec<BoxData>
where
    F: Fn(usize, usize) -> [f32; 3] + Copy,
{
    let step = box_size + spacing;
    let offset_x = (count_x as f32 - 1.0) * step * 0.5;
    let offset_z = (count_z as f32 - 1.0) * step * 0.5;

    (0..count_x)
        .flat_map(|x| {
            (0..count_z).map(move |z| {
                let pos = Vec3::new(
                    center.x + x as f32 * step - offset_x,
                    center.y,
                    center.z + z as f32 * step - offset_z,
                );
                box_at(pos, Vec3::new(box_size, height, box_size), colors(x, z))
            }).collect::<Vec<_>>()
        })
        .collect()
}

/// Creates a circular ring of boxes
pub fn ring(
    center: Vec3,
    radius: f32,
    count: usize,
    box_size: Vec3,
    colors: impl Fn(usize) -> [f32; 3],
) -> Vec<BoxData> {
    (0..count)
        .map(|i| {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let pos = center + Vec3::new(
                angle.cos() * radius,
                0.0,
                angle.sin() * radius,
            );
            box_at(pos, box_size, colors(i))
        })
        .collect()
}

/// Creates multiple concentric rings
pub fn rings(
    center: Vec3,
    base_radius: f32,
    radius_step: f32,
    ring_count: usize,
    boxes_per_ring: impl Fn(usize) -> usize,
    box_size: impl Fn(usize) -> Vec3,
    colors: impl Fn(usize, usize) -> [f32; 3],
) -> Vec<BoxData> {
    (0..ring_count)
        .flat_map(|ring_idx| {
            let radius = base_radius + ring_idx as f32 * radius_step;
            let count = boxes_per_ring(ring_idx);
            ring(
                center,
                radius,
                count,
                box_size(ring_idx),
                |i| colors(ring_idx, i),
            )
        })
        .collect()
}

/// Creates a spiral of boxes
pub fn spiral(
    center: Vec3,
    start_radius: f32,
    end_radius: f32,
    height_per_turn: f32,
    turns: f32,
    boxes_per_turn: usize,
    box_size: Vec3,
    colors: impl Fn(usize) -> [f32; 3],
) -> Vec<BoxData> {
    let total_boxes = (turns * boxes_per_turn as f32) as usize;

    (0..total_boxes)
        .map(|i| {
            let t = i as f32 / total_boxes as f32;
            let angle = t * turns * std::f32::consts::TAU;
            let radius = start_radius + (end_radius - start_radius) * t;
            let y = t * turns * height_per_turn;

            let pos = center + Vec3::new(
                angle.cos() * radius,
                y,
                angle.sin() * radius,
            );
            box_at(pos, box_size, colors(i))
        })
        .collect()
}

/// Creates a wall of boxes
pub fn wall<F>(
    position: Vec3,
    direction: WallDirection,
    width: f32,
    height: f32,
    thickness: f32,
    box_size: f32,
    spacing: f32,
    colors: F,
) -> Vec<BoxData>
where
    F: Fn(usize, usize) -> [f32; 3] + Copy,
{
    let step = box_size + spacing;
    let boxes_wide = (width / step) as usize;
    let boxes_high = (height / step) as usize;

    (0..boxes_high)
        .flat_map(|y| {
            (0..boxes_wide).map(move |x| {
                let local_pos = match direction {
                    WallDirection::NorthSouth => Vec3::new(
                        x as f32 * step - width * 0.5,
                        y as f32 * step,
                        0.0,
                    ),
                    WallDirection::EastWest => Vec3::new(
                        0.0,
                        y as f32 * step,
                        x as f32 * step - width * 0.5,
                    ),
                };

                let size = match direction {
                    WallDirection::NorthSouth => Vec3::new(box_size, box_size, thickness),
                    WallDirection::EastWest => Vec3::new(thickness, box_size, box_size),
                };

                box_at(position + local_pos, size, colors(x, y))
            }).collect::<Vec<_>>()
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub enum WallDirection {
    NorthSouth,
    EastWest,
}

/// Creates a room with four walls
pub fn room<F>(
    center: Vec3,
    size: f32,
    wall_height: f32,
    wall_thickness: f32,
    box_size: f32,
    spacing: f32,
    colors: F,
) -> Vec<BoxData>
where
    F: Fn(usize, usize, usize) -> [f32; 3], // wall_index, x, y
{
    let half_size = size * 0.5;

    let walls = [
        (Vec3::new(center.x, center.y, center.z - half_size), WallDirection::EastWest, 0),
        (Vec3::new(center.x, center.y, center.z + half_size), WallDirection::EastWest, 1),
        (Vec3::new(center.x - half_size, center.y, center.z), WallDirection::NorthSouth, 2),
        (Vec3::new(center.x + half_size, center.y, center.z), WallDirection::NorthSouth, 3),
    ];

    walls.iter()
        .flat_map(|(pos, dir, wall_idx)| {
            wall(*pos, *dir, size, wall_height, wall_thickness, box_size, spacing,
                |x, y| colors(*wall_idx, x, y))
        })
        .collect()
}

// ============================================================================
// Color Generators - Create color schemes
// ============================================================================

/// Generates rainbow colors based on index
pub fn rainbow_gradient(total: usize) -> impl Fn(usize) -> [f32; 3] {
    move |i| hsv_to_rgb(i as f32 / total as f32, 0.8, 0.9)
}

/// Generates a single solid color
pub fn solid_color(color: [f32; 3]) -> impl Fn(usize) -> [f32; 3] {
    move |_| color
}

/// Generates alternating colors
pub fn alternating_colors(color1: [f32; 3], color2: [f32; 3]) -> impl Fn(usize) -> [f32; 3] {
    move |i| if i % 2 == 0 { color1 } else { color2 }
}

/// Generates a gradient between two colors
pub fn gradient(color1: [f32; 3], color2: [f32; 3], steps: usize) -> impl Fn(usize) -> [f32; 3] {
    move |i| {
        let t = (i % steps) as f32 / steps as f32;
        [
            color1[0] + (color2[0] - color1[0]) * t,
            color1[1] + (color2[1] - color1[1]) * t,
            color1[2] + (color2[2] - color1[2]) * t,
        ]
    }
}

// ============================================================================
// Transformation Functions - Modify existing boxes
// ============================================================================

/// Makes all boxes in a collection reflective
pub fn make_reflective(boxes: Vec<BoxData>, reflectivity: f32) -> Vec<BoxData> {
    boxes.into_iter()
        .map(|mut b| {
            b.reflectivity = reflectivity;
            b
        })
        .collect()
}

/// Translates all boxes by an offset
pub fn translate(boxes: Vec<BoxData>, offset: Vec3) -> Vec<BoxData> {
    boxes.into_iter()
        .map(|mut b| {
            let min = Vec3::from_array(b.min) + offset;
            let max = Vec3::from_array(b.max) + offset;
            b.min = min.to_array();
            b.max = max.to_array();
            b.center0 = (Vec3::from_array(b.center0) + offset).to_array();
            b.center1 = (Vec3::from_array(b.center1) + offset).to_array();
            b
        })
        .collect()
}

/// Scales all boxes by a factor around a center point
pub fn scale(boxes: Vec<BoxData>, center: Vec3, factor: f32) -> Vec<BoxData> {
    boxes.into_iter()
        .map(|mut b| {
            let min = (Vec3::from_array(b.min) - center) * factor + center;
            let max = (Vec3::from_array(b.max) - center) * factor + center;
            b.min = min.to_array();
            b.max = max.to_array();
            b.center0 = ((Vec3::from_array(b.center0) - center) * factor + center).to_array();
            b.center1 = ((Vec3::from_array(b.center1) - center) * factor + center).to_array();
            b.half_size = (Vec3::from_array(b.half_size) * factor).to_array();
            b
        })
        .collect()
}

// ============================================================================
// DemoBuilder - Fluent API for scene construction
// ============================================================================

/// Builder for creating demo scenes with a fluent API
pub struct DemoBuilder {
    boxes: Vec<BoxData>,
}

impl DemoBuilder {
    /// Creates a new empty demo builder
    pub fn new() -> Self {
        Self { boxes: Vec::new() }
    }

    /// Adds a ground plane
    pub fn add_ground(mut self, color: [f32; 3]) -> Self {
        self.boxes.push(ground(
            [-200.0, -1.0, -200.0],
            [200.0, -0.99, 200.0],
            color,
        ));
        self
    }

    /// Adds a reflective ground plane
    pub fn add_reflective_ground(mut self, color: [f32; 3], reflectivity: f32) -> Self {
        self.boxes.push(reflective_ground(
            [-200.0, -1.0, -200.0],
            [200.0, -0.99, 200.0],
            color,
            reflectivity,
        ));
        self
    }

    /// Adds a single box
    pub fn add_box(mut self, position: Vec3, size: Vec3, color: [f32; 3]) -> Self {
        self.boxes.push(box_at(position, size, color));
        self
    }

    /// Adds a reflective box
    pub fn add_reflective_box(mut self, position: Vec3, size: Vec3, color: [f32; 3], reflectivity: f32) -> Self {
        self.boxes.push(reflective_box_at(position, size, color, reflectivity));
        self
    }

    /// Adds a moving box
    pub fn add_moving_box(mut self, size: Vec3, start: Vec3, end: Vec3, color: [f32; 3]) -> Self {
        self.boxes.push(BoxData::create_moving_box(size, start, end, color));
        self
    }

    /// Adds a grid of boxes
    pub fn add_grid(
        mut self,
        center: Vec3,
        box_size: f32,
        spacing: f32,
        count_x: usize,
        count_z: usize,
        height: f32,
        colors: impl Fn(usize, usize) -> [f32; 3] + Copy,
    ) -> Self {
        self.boxes.extend(grid(center, box_size, spacing, count_x, count_z, height, colors));
        self
    }

    /// Adds a circular ring of boxes
    pub fn add_ring(
        mut self,
        radius: f32,
        count: usize,
        height: f32,
        colors: impl Fn(usize) -> [f32; 3],
    ) -> Self {
        let size = Vec3::new(2.0, height, 2.0);
        self.boxes.extend(ring(Vec3::ZERO, radius, count, size, colors));
        self
    }

    /// Adds multiple concentric rings
    pub fn add_rings(
        mut self,
        base_radius: f32,
        radius_step: f32,
        ring_count: usize,
        boxes_per_ring: impl Fn(usize) -> usize,
        box_size: impl Fn(usize) -> Vec3,
        colors: impl Fn(usize, usize) -> [f32; 3],
    ) -> Self {
        self.boxes.extend(rings(
            Vec3::ZERO,
            base_radius,
            radius_step,
            ring_count,
            boxes_per_ring,
            box_size,
            colors,
        ));
        self
    }

    /// Adds a spiral of boxes
    pub fn add_spiral(
        mut self,
        start_radius: f32,
        end_radius: f32,
        height_per_turn: f32,
        turns: f32,
        boxes_per_turn: usize,
        box_size: Vec3,
        colors: impl Fn(usize) -> [f32; 3],
    ) -> Self {
        self.boxes.extend(spiral(
            Vec3::ZERO,
            start_radius,
            end_radius,
            height_per_turn,
            turns,
            boxes_per_turn,
            box_size,
            colors,
        ));
        self
    }

    /// Adds a wall
    pub fn add_wall(
        mut self,
        position: Vec3,
        direction: WallDirection,
        width: f32,
        height: f32,
        thickness: f32,
        box_size: f32,
        spacing: f32,
        colors: impl Fn(usize, usize) -> [f32; 3] + Copy,
    ) -> Self {
        self.boxes.extend(wall(position, direction, width, height, thickness, box_size, spacing, colors));
        self
    }

    /// Adds a room with four walls
    pub fn add_room(
        mut self,
        center: Vec3,
        size: f32,
        wall_height: f32,
        wall_thickness: f32,
        box_size: f32,
        spacing: f32,
        colors: impl Fn(usize, usize, usize) -> [f32; 3],
    ) -> Self {
        self.boxes.extend(room(center, size, wall_height, wall_thickness, box_size, spacing, colors));
        self
    }

    /// Adds custom boxes from any iterator
    pub fn add_custom(mut self, boxes: impl IntoIterator<Item = BoxData>) -> Self {
        self.boxes.extend(boxes);
        self
    }

    /// Applies a transformation to all existing boxes
    pub fn transform(mut self, f: impl Fn(Vec<BoxData>) -> Vec<BoxData>) -> Self {
        self.boxes = f(self.boxes);
        self
    }

    /// Makes all existing boxes reflective
    pub fn make_all_reflective(self, reflectivity: f32) -> Self {
        self.transform(|boxes| make_reflective(boxes, reflectivity))
    }

    /// Translates all existing boxes
    pub fn translate_all(self, offset: Vec3) -> Self {
        self.transform(|boxes| translate(boxes, offset))
    }

    /// Scales all existing boxes
    pub fn scale_all(self, center: Vec3, factor: f32) -> Self {
        self.transform(|boxes| scale(boxes, center, factor))
    }

    /// Returns the number of boxes in the scene
    pub fn count(&self) -> usize {
        self.boxes.len()
    }

    /// Builds the final scene
    pub fn build(self) -> Vec<BoxData> {
        println!("Demo scene created: {} total boxes", self.boxes.len());
        self.boxes
    }
}

impl Default for DemoBuilder {
    fn default() -> Self {
        Self::new()
    }
}
