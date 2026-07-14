use crate::inputs::action::Action;
use std::collections::HashMap;
use winit::keyboard::KeyCode;

pub struct KeyBindings {
    pub keyboard_bindings: HashMap<KeyCode, Action>,
    pub controller_bindings: HashMap<Action, Vec<KeyCode>>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            keyboard_bindings: HashMap::from([
                (KeyCode::KeyW, Action::MoveForward),
                (KeyCode::KeyS, Action::Movebackwards),
                (KeyCode::KeyD, Action::MoveRight),
                (KeyCode::KeyA, Action::MoveLeft),
                (KeyCode::Space, Action::MoveUp),
                (KeyCode::ShiftLeft, Action::MoveDown),
            ]),

            controller_bindings: HashMap::new(), // TODO empty for now add defaults later
        }
    }
}

impl KeyBindings {
    pub fn get_action(&self, key: &KeyCode) -> Option<Action> {
        self.keyboard_bindings.get(key).copied()
    }
}
