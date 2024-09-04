use crate::prelude::*;
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};

const DEBOUNCE_THRESHOLD: u32 = 2;

#[derive(Clone, Debug)]
pub enum Action {
    Key(char),
    Sound(f32),
    None,
}

impl Action {
    pub fn to_string(&self) -> String {
        match self {
            Action::Key(key) => format!("Key \"{key}\"").to_string(),
            Action::Sound(_) => String::from("Sound"),
            Action::None => String::from("(no action)"),
        }
    }
}

#[derive(Default)]
pub struct InputState {
    pub enabled: bool,
    pub input: AbstractionLayer,
    pub debounce_count: u32,
    pub active_prediction: u8,
    pub last_prediction: u8,
    pub actions: Vec<Action>,
    pub tap: Vec<bool>,
    pub verbose: bool,
}

impl InputState {
    pub fn new(verbose: bool) -> Self {
        let mut obj = Self::default();
        obj.verbose = verbose;
        obj.actions = vec![
            Action::None,
            Action::Key('w'),
            Action::Key('a'),
            Action::Key('d'),
            Action::Key('s'),
        ];
        obj.tap = vec![false, false, false, false, false];
        obj
    }

    pub fn reset(&mut self) {
        self.release(self.active_prediction as usize);
        self.last_prediction = 0;
        self.active_prediction = 0;
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn set_action(&mut self, index: usize, action: Action) {
        if index < self.actions.len() {
            self.actions[index] = action;
        } else if self.verbose {
            println!(
                "Failed to assign action, index too large. Index: {index}, action vector size: {}",
                self.actions.len()
            );
        }
    }

    pub fn set_tap(&mut self, index: usize, tap: bool) {
        if index < self.tap.len() {
            self.tap[index] = tap;
        } else if self.verbose {
            println!(
                "Failed to assign tap flag, index too large. Index: {index}, tap vector size: {}",
                self.tap.len()
            );
        }
    }

    pub fn set_predicted(&mut self, prediction: u8) {
        if !self.enabled {
            return;
        }

        if self.active_prediction == prediction {
            // If the same key is predicted that's already being pressed,
            // we don't need to do anything.
            self.last_prediction = prediction;
            return;
        }

        self.debounce_count = if self.last_prediction == prediction {
            // If the key is predicted multiple times in a row, lower the debounce count.
            self.debounce_count.saturating_sub(1)
        } else {
            // If a different key is predicted, reset the debounce count to the max,
            // which will effectively pause the key pressing/releasing, to prevent
            // oscillating predictions from generating lots of key press/release noise.
            DEBOUNCE_THRESHOLD
        };

        self.last_prediction = prediction;

        // If the same key has been predicted often enough in a row without oscillations,
        // we can confidently release the old key and press the new key (if any).
        if self.debounce_count == 0 {
            self.release(self.active_prediction as usize);
            self.press(prediction as usize);
            self.active_prediction = prediction;
        }
    }

    fn press(&mut self, index: usize) {
        // This will perform the action as given by index
        match self.actions.get(index) {
            Some(Action::Key(key)) => {
                let tap = self.tap.get(index).unwrap_or(&false);
                self.input.press(*key, *tap);
                if self.verbose {
                    println!("Pressing {key}");
                }
            }
            Some(Action::Sound(frequency)) => {
                sound::play(*frequency);
                if self.verbose {
                    println!("Playing sound freq {frequency}Hz");
                }
            }
            _ => {}
        }
    }

    fn release(&mut self, index: usize) {
        // This will "release" the action as given by index
        if let Some(Action::Key(key)) = self.actions.get(index) {
            let tap = self.tap.get(index).unwrap_or(&false);
            if !tap {
                self.input.release(*key);
                if self.verbose {
                    println!("Releasing {key}");
                }
            }
        }
    }
}

pub struct AbstractionLayer {
    enigo: Option<Enigo>,
}

impl Default for AbstractionLayer {
    fn default() -> Self {
        let tryenigo = Enigo::new(&Settings::default());
        if let Ok(enigo) = tryenigo {
            Self { enigo: Some(enigo) }
        } else {
            println!("Error: Could not initialize enigo library for simulation of key presses.");
            Self { enigo: None }
        }
    }
}

impl AbstractionLayer {
    pub fn press(&mut self, key: char, tap: bool) {
        if self.enigo.is_some() {
            let activity = if tap { Click } else { Press };
            self.enigo
                .as_mut()
                .unwrap()
                .key(Key::Unicode(key), activity)
                .expect("Key press failed");
        }
    }

    pub fn release(&mut self, key: char) {
        if self.enigo.is_some() {
            self.enigo
                .as_mut()
                .unwrap()
                .key(Key::Unicode(key), Release)
                .expect("Key press failed");
        }
    }
}
