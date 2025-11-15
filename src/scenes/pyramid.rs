use crate::types::{BoxData, TriangleData};

/// Creates a simple pyramid scene with actual triangles for testing
/// Returns a tuple of (boxes, triangles)
pub fn create_pyramid_scene() -> Vec<BoxData> {
    let mut boxes = Vec::new();

    // Ground plane (2 triangles forming a square)
    // Note: For now, we'll use a box for the ground since the system expects boxes
    boxes.push(BoxData::new(
        [-10.0, -0.5, -10.0],
        [10.0, 0.0, 10.0],
        [0.3, 0.3, 0.3], // Dark gray ground
    ));

    println!("Pyramid scene created: {} boxes (ground only, pyramid will be triangles)", boxes.len());
    boxes
}

/// Creates triangle data for a pyramid
/// A square pyramid with 4 triangular sides + 2 triangles for the square base = 6 triangles
pub fn create_pyramid_triangles() -> Vec<TriangleData> {
    let mut triangles = Vec::new();

    // Pyramid apex (top point)
    let apex = [0.0, 5.0, 0.0];

    // Base square corners
    let base_size = 4.0;
    let base_y = 0.0;
    let p0 = [-base_size, base_y, -base_size]; // Front-left
    let p1 = [base_size, base_y, -base_size];  // Front-right
    let p2 = [base_size, base_y, base_size];   // Back-right
    let p3 = [-base_size, base_y, base_size];  // Back-left

    // UV coordinates (simple)
    let uv0 = [0.0, 0.0];
    let uv1 = [1.0, 0.0];
    let uv2 = [0.5, 1.0];

    // 4 triangular sides (different colors via material_id)
    // Front face (red)
    triangles.push(TriangleData::new(p0, p1, apex, uv0, uv1, uv2, 0));

    // Right face (green)
    triangles.push(TriangleData::new(p1, p2, apex, uv0, uv1, uv2, 1));

    // Back face (blue)
    triangles.push(TriangleData::new(p2, p3, apex, uv0, uv1, uv2, 2));

    // Left face (yellow)
    triangles.push(TriangleData::new(p3, p0, apex, uv0, uv1, uv2, 3));

    // Base (2 triangles, gray)
    let uv_base = [0.0, 0.0];
    triangles.push(TriangleData::new(p0, p2, p1, uv_base, uv_base, uv_base, 4));
    triangles.push(TriangleData::new(p0, p3, p2, uv_base, uv_base, uv_base, 4));

    println!("Pyramid triangles created: {}", triangles.len());
    triangles
}
