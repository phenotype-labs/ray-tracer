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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_button_equality() {
        assert_eq!(Button::KeyW, Button::KeyW);
        assert_eq!(Button::Space, Button::Space);
        assert_ne!(Button::KeyW, Button::KeyA);
    }

    #[test]
    fn test_button_clone() {
        let btn1 = Button::KeyW;
        let btn2 = btn1.clone();
        assert_eq!(btn1, btn2);
    }

    #[test]
    fn test_button_copy() {
        let btn1 = Button::Space;
        let btn2 = btn1; // Copy, not move
        assert_eq!(btn1, btn2);
    }

    #[test]
    fn test_button_debug() {
        let debug_str = format!("{:?}", Button::KeyW);
        assert_eq!(debug_str, "KeyW");

        let debug_str = format!("{:?}", Button::MouseLeft);
        assert_eq!(debug_str, "MouseLeft");
    }

    #[test]
    fn test_button_hash() {
        let mut set = HashSet::new();
        set.insert(Button::KeyW);
        set.insert(Button::KeyA);
        set.insert(Button::Space);

        assert!(set.contains(&Button::KeyW));
        assert!(set.contains(&Button::KeyA));
        assert!(!set.contains(&Button::KeyS));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_button_hash_duplicates() {
        let mut set = HashSet::new();
        set.insert(Button::KeyW);
        set.insert(Button::KeyW); // Duplicate

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_all_button_variants_unique() {
        let all_buttons = vec![
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

        let set: HashSet<_> = all_buttons.iter().collect();
        assert_eq!(set.len(), 11);
    }

    // Test mock controller implementation
    struct MockController {
        pressed: Vec<Button>,
    }

    impl Controller for MockController {
        fn is_down(&self, button: Button) -> bool {
            self.pressed.contains(&button)
        }

        fn get_down_keys(&self) -> &[Button] {
            &self.pressed
        }
    }

    #[test]
    fn test_controller_is_down() {
        let controller = MockController {
            pressed: vec![Button::KeyW, Button::Space],
        };

        assert!(controller.is_down(Button::KeyW));
        assert!(controller.is_down(Button::Space));
        assert!(!controller.is_down(Button::KeyA));
    }

    #[test]
    fn test_controller_get_down_keys() {
        let controller = MockController {
            pressed: vec![Button::KeyW, Button::KeyA, Button::Space],
        };

        let down_keys = controller.get_down_keys();
        assert_eq!(down_keys.len(), 3);
        assert!(down_keys.contains(&Button::KeyW));
        assert!(down_keys.contains(&Button::KeyA));
        assert!(down_keys.contains(&Button::Space));
    }

    #[test]
    fn test_controller_no_keys_pressed() {
        let controller = MockController { pressed: vec![] };

        assert!(!controller.is_down(Button::KeyW));
        assert!(!controller.is_down(Button::Space));
        assert_eq!(controller.get_down_keys().len(), 0);
    }

    #[test]
    fn test_controller_all_keys_pressed() {
        let controller = MockController {
            pressed: vec![
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
            ],
        };

        assert_eq!(controller.get_down_keys().len(), 11);
        for button in controller.get_down_keys() {
            assert!(controller.is_down(*button));
        }
    }
}
