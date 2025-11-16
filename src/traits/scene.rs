use crate::types::TriangleData;

/// Scene construction and modification abstraction
pub trait SceneProvider {
    /// Build the geometry for this scene
    fn build_geometry(&self) -> Vec<TriangleData>;

    /// Update scene state (optional, for animated scenes)
    fn update(&mut self, _time: f32) {}

    /// Get scene name for debugging
    fn name(&self) -> &str {
        "Scene"
    }
}
