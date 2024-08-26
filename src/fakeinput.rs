use enigo::{
    Direction::{Press, Release},
    Enigo, Key, Keyboard, Settings,
};
// use std::collections::HashSet;

const DEBOUNCE_THRESHOLD: u32 = 2;

#[derive(Default)]
pub struct InputState {
    pub input: AbstractionLayer,
    pub debounce_count: u32,
    pub active_prediction: u8,
    pub last_prediction: u8,
    pub actions: Vec<Option<char>>,
}

impl InputState {
    pub fn new() -> Self {
        let mut obj = Self::default();
        obj.actions = vec![None, Some('a')];
        obj
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
            if let Some(Some(key)) = self.actions.get(self.active_prediction as usize) {
                self.input.release(*key);
                println!("releasing {key}");
            }
            if let Some(Some(key)) = self.actions.get(prediction as usize) {
                self.input.press(*key);
                println!("pressing {key}");
            }
            self.active_prediction = prediction;
        }
    }
}

pub struct AbstractionLayer {
    enigo: Enigo,
}

impl Default for AbstractionLayer {
    fn default() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).unwrap(),
        }
    }
}

impl AbstractionLayer {
    pub fn press(&mut self, key: char) {
        self.enigo.key(Key::Unicode(key), Press).expect("Key press failed");
    }

    pub fn release(&mut self, key: char) {
        self.enigo.key(Key::Unicode(key), Release).expect("Key press failed");
    }
}
