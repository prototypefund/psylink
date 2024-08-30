use crate::prelude::*;
use enigo::{
    Direction::{Press, Release},
    Enigo, Key, Keyboard, Settings,
};

const DEBOUNCE_THRESHOLD: u32 = 2;

#[derive(Debug)]
pub enum Action {
    Key(char),
    Sound(f32),
    None,
}

#[derive(Default)]
pub struct InputState {
    pub input: AbstractionLayer,
    pub debounce_count: u32,
    pub active_prediction: u8,
    pub last_prediction: u8,
    pub actions: Vec<Action>,
    pub verbose: bool,
}

impl InputState {
    pub fn new(verbose: bool) -> Self {
        let mut obj = Self::default();
        obj.verbose = verbose;
        obj.actions = vec![Action::None, Action::Key('w'), Action::Key('a'), Action::Key('d'), Action::Key('s')];
        obj
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

    pub fn set_predicted(&mut self, prediction: u8) {
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
            if let Some(Action::Key(key)) = self.actions.get(self.active_prediction as usize) {
                self.input.release(*key);
                if self.verbose {
                    println!("Releasing {key}");
                }
            }
            match self.actions.get(prediction as usize) {
                Some(Action::Key(key)) => {
                    self.input.press(*key);
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
            self.active_prediction = prediction;
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
    pub fn press(&mut self, key: char) {
        if self.enigo.is_some() {
            self.enigo
                .as_mut()
                .unwrap()
                .key(Key::Unicode(key), Press)
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
