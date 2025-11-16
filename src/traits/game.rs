use super::controller::Controller;

/// Game abstraction - handles game lifecycle and logic
pub trait Game {
    /// Register display with the game
    fn register_display(&mut self);

    /// Register controller with the game
    fn register_controller<C: Controller>(&mut self, controller: &C);
}
