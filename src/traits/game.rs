use super::controller::Controller;
use super::executor::PipelineExecutor;

/// Game abstraction - handles game lifecycle and logic
pub trait Game {
    /// Register executor with the game
    fn register_executor<E: PipelineExecutor>(&mut self, executor: &mut E);

    /// Register controller with the game
    fn register_controller<C: Controller>(&mut self, controller: &C);
}
