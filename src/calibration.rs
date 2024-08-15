pub struct Sample {
    pub features: Vec<f64>,
    pub label: u8,
}

pub struct Samples {
    pub samples: Vec<Sample>,
}

#[derive(Clone, Default)]
pub struct Calibrator {
    pub action_count: usize,
    pub current_action: usize,
    pub remaining_key_presses: Vec<u32>,
    pub timer: f64,
    pub state: CalibratorState,
}

#[derive(Clone, Default, PartialEq)]
pub enum CalibratorState {
    #[default]
    Init,
    Welcome,
    NullActionWait,
    NullAction,
    GestureActionWait,
    GestureAction,
    Done,
}

impl Calibrator {
    pub fn new(action_count: usize, key_presses: u32) -> Self {
        let mut result = Self::default();
        result.action_count = action_count;
        for _ in 0..action_count {
            result.remaining_key_presses.push(key_presses);
        }
        result
    }

    /// returns true if a state change happened
    pub fn tick(&mut self, time: f64) -> bool {
        if self.timer > 0.0 {
            self.timer -= time;
        }
        if self.timer <= 0.0 {
            let new_state = match self.state {
                CalibratorState::Init => CalibratorState::Welcome,
                CalibratorState::Welcome => CalibratorState::NullActionWait,
                CalibratorState::NullActionWait => CalibratorState::NullAction,
                CalibratorState::NullAction => {
                    if self.remaining_key_presses.iter().all(|&x| x <= 0) {
                        CalibratorState::Done
                    } else {
                        CalibratorState::GestureActionWait
                    }
                }
                CalibratorState::GestureActionWait => CalibratorState::GestureAction,
                CalibratorState::GestureAction => {
                    self.remaining_key_presses[self.current_action] =
                        self.remaining_key_presses[self.current_action].saturating_sub(1);
                    self.current_action = (self.current_action + 1) % self.action_count;

                    CalibratorState::NullActionWait
                }
                CalibratorState::Done => CalibratorState::Done,
            };
            let delay = match new_state {
                CalibratorState::Init | CalibratorState::Done => 0.0,
                CalibratorState::Welcome => 4.0,
                CalibratorState::NullActionWait => 3.0,
                CalibratorState::NullAction => 8.0,
                CalibratorState::GestureActionWait => 4.0,
                CalibratorState::GestureAction => 8.0,
            };

            let state_change_happened = self.state != new_state;
            self.state = new_state;
            self.timer = delay;
            return state_change_happened;
        } else {
            return false;
        }
    }

    pub fn generate_message(&self) -> String {
        let current_action = self.current_action.saturating_add(1);
        match self.state {
            CalibratorState::Init => "Initializing...".into(),
            CalibratorState::Welcome => {
                "Calibration starting. Please follow the instructions.".into()
            }
            CalibratorState::NullActionWait => "Prepare to rest your arm.".into(),
            CalibratorState::NullAction => "Rest your arm now.".into(),
            CalibratorState::GestureActionWait => format!("Prepare movement #{current_action}"),
            CalibratorState::GestureAction => format!("Do movement #{current_action} now."),
            CalibratorState::Done => "Calibration complete.".into(),
        }
    }
}
