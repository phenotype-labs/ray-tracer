/// Camera movement and control abstraction
pub trait CameraController {
    /// Update camera state based on elapsed time
    fn update(&mut self, delta_time: f32);

    /// Get the view matrix for rendering
    fn view_matrix(&self) -> [[f32; 4]; 4];

    /// Get the camera position in world space
    fn position(&self) -> [f32; 3];

    /// Get the camera forward direction
    fn forward(&self) -> [f32; 3];
}
