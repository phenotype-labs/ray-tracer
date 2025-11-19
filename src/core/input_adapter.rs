use std::collections::HashSet;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use super::controller::{Button, Controller};

/// Adapter that bridges Winit events to the Controller trait
#[derive(Debug, Clone)]
pub struct WinitController {
    /// Currently pressed buttons
    pressed_keys: HashSet<Button>,
    /// All pressed buttons as a vec (for efficient get_down_keys)
    pressed_vec: Vec<Button>,
    /// Current mouse position (relative to window)
    mouse_position: Option<(f32, f32)>,
    /// Mouse movement delta since last reset
    mouse_delta: (f32, f32),
}

impl WinitController {
    /// Create a new WinitController with no pressed keys
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            pressed_vec: Vec::new(),
            mouse_position: None,
            mouse_delta: (0.0, 0.0),
        }
    }

    /// Process a Winit WindowEvent and update internal state
    pub fn process_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    if let Some(button) = Self::keycode_to_button(keycode) {
                        match event.state {
                            ElementState::Pressed => {
                                if self.pressed_keys.insert(button) {
                                    self.pressed_vec.push(button);
                                }
                            }
                            ElementState::Released => {
                                if self.pressed_keys.remove(&button) {
                                    self.pressed_vec.retain(|&b| b != button);
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(btn) = Self::mouse_button_to_button(*button) {
                    match state {
                        ElementState::Pressed => {
                            if self.pressed_keys.insert(btn) {
                                self.pressed_vec.push(btn);
                            }
                        }
                        ElementState::Released => {
                            if self.pressed_keys.remove(&btn) {
                                self.pressed_vec.retain(|&b| b != btn);
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = (position.x as f32, position.y as f32);
                if let Some(old_pos) = self.mouse_position {
                    let delta = (new_pos.0 - old_pos.0, new_pos.1 - old_pos.1);
                    self.mouse_delta.0 += delta.0;
                    self.mouse_delta.1 += delta.1;
                }
                self.mouse_position = Some(new_pos);
            }
            _ => {}
        }
    }

    /// Reset per-frame state (mouse delta)
    /// Call this at the end of each frame after processing input
    pub fn reset_deltas(&mut self) {
        self.mouse_delta = (0.0, 0.0);
    }

    /// Get current mouse position (if available)
    pub fn mouse_position(&self) -> Option<(f32, f32)> {
        self.mouse_position
    }

    /// Get accumulated mouse delta since last reset
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    /// Map Winit KeyCode to Button
    fn keycode_to_button(keycode: KeyCode) -> Option<Button> {
        match keycode {
            KeyCode::KeyW => Some(Button::KeyW),
            KeyCode::KeyA => Some(Button::KeyA),
            KeyCode::KeyS => Some(Button::KeyS),
            KeyCode::KeyD => Some(Button::KeyD),
            KeyCode::KeyQ => Some(Button::KeyQ),
            KeyCode::KeyE => Some(Button::KeyE),
            KeyCode::Space => Some(Button::Space),
            KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(Button::Shift),
            KeyCode::Escape => Some(Button::Escape),
            _ => None,
        }
    }

    /// Map Winit MouseButton to Button
    fn mouse_button_to_button(button: MouseButton) -> Option<Button> {
        match button {
            MouseButton::Left => Some(Button::MouseLeft),
            MouseButton::Right => Some(Button::MouseRight),
            _ => None,
        }
    }
}

impl Default for WinitController {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller for WinitController {
    fn is_down(&self, button: Button) -> bool {
        self.pressed_keys.contains(&button)
    }

    fn get_down_keys(&self) -> &[Button] {
        &self.pressed_vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Winit event construction requires internal fields that are not publicly accessible
    // These tests verify the Controller trait implementation and basic functionality

    #[test]
    fn test_new_controller_empty() {
        let controller = WinitController::new();
        assert!(!controller.is_down(Button::KeyW));
        assert_eq!(controller.get_down_keys().len(), 0);
        assert_eq!(controller.mouse_position(), None);
        assert_eq!(controller.mouse_delta(), (0.0, 0.0));
    }

    #[test]
    fn test_default_controller() {
        let controller = WinitController::default();
        assert!(!controller.is_down(Button::KeyW));
        assert_eq!(controller.get_down_keys().len(), 0);
    }

    #[test]
    fn test_delta_reset() {
        let mut controller = WinitController::new();
        // Set some delta manually (simulating mouse movement)
        controller.mouse_delta = (10.0, 5.0);
        controller.mouse_position = Some((100.0, 200.0));

        controller.reset_deltas();
        assert_eq!(controller.mouse_delta(), (0.0, 0.0));
        // Position should remain
        assert_eq!(controller.mouse_position(), Some((100.0, 200.0)));
    }

    #[test]
    fn test_button_mapping() {
        // Test that Button enum variants exist and can be used
        let buttons = vec![
            Button::KeyW,
            Button::KeyA,
            Button::KeyS,
            Button::KeyD,
            Button::KeyQ,
            Button::KeyE,
            Button::Space,
            Button::Shift,
            Button::Escape,
            Button::MouseLeft,
            Button::MouseRight,
        ];

        let controller = WinitController::new();
        for button in buttons {
            assert!(!controller.is_down(button));
        }
    }
}
