/// Input button identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    KeyW,
    KeyA,
    KeyS,
    KeyD,
    KeyQ,
    KeyE,
    Space,
    Shift,
    Escape,
    MouseLeft,
    MouseRight,
}

/// Controller - handles button input states
pub trait Controller {
    /// Check if button is currently down
    fn is_down(&self, button: Button) -> bool;

    /// Get all currently pressed buttons
    fn get_down_keys(&self) -> &[Button];
}
