use crate::prelude::*;
use plotters::prelude::*;
use slint::SharedPixelBuffer;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
slint::include_modules!();

const MAX_POINTS: usize = 2048;

pub async fn start(app: App) {
    let ui = MainWindow::new().unwrap();

    let calibrator = Arc::new(Mutex::new(Calibrator::default()));
    let calibrationstate = Arc::new(Mutex::new(false));
    let keystate = Arc::new(Mutex::new(HashSet::<String>::new()));
    let keystate_clone_writer = Arc::clone(&keystate);
    ui.global::<Logic>()
        .on_key_handler(move |key: slint::SharedString, pressed: bool| {
            let mut keystate = keystate_clone_writer.lock().unwrap();
            if pressed {
                keystate.insert(key.to_string());
            } else {
                keystate.remove(&key.to_string());
            }
        });

    let ui_weak = ui.as_weak();
    let calibrationstate_clone_writer = Arc::clone(&calibrationstate);
    let calibrator_clone_writer = Arc::clone(&calibrator);
    ui.global::<Logic>()
        .on_start_calibration_handler(move |actions: i32| {
            let mut calibrationstate = calibrationstate_clone_writer.lock().unwrap();
            *calibrationstate = true;
            let mut calibrator = calibrator_clone_writer.lock().unwrap();
            *calibrator = Calibrator::new(actions as usize, 3);
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_calibrating(true);
                ui.set_text_calibration_instruction(format!("Attempting to calibrate...").into());
            });
        });

    let ui_weak = ui.as_weak();
    let calibrationstate_clone_writer = Arc::clone(&calibrationstate);
    ui.global::<Logic>().on_stop_calibration_handler(move || {
        let mut calibrationstate = calibrationstate_clone_writer.lock().unwrap();
        *calibrationstate = false;
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_calibrating(false);
            ui.set_text_calibration_instruction(format!("No calibration in progress.").into());
        });
    });

    let appclone = app.clone();
    let ui_weak = ui.as_weak();
    tokio::spawn(async move {
        let mut device = loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            if let Ok(device) = bluetooth::find_peripheral(appclone).await {
                let address = device.address.clone();
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_text_connection_title(
                        format!("Found PsyLink with MAC address {address}.\n\nConnecting...")
                            .into(),
                    );
                });
                break device;
            }
        };
        device.find_characteristics().await;

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_connected(true);
            ui.set_text_connection_title(
                format!("PsyLink connection established.\n\nPlease select another tab.").into(),
            );
            ui.set_text_graph_title(format!("Displaying PsyLink signals.").into());
            ui.set_page(1);
        });
        let mut decoder = protocol::Decoder::new(8);
        let mut plotter = Plotter::new(8);

        loop {
            let bytearray: Vec<u8> = device.read().await.unwrap(); // TODO: catch panic
            let text = bytearray
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            if appclone.verbose > 1 {
                println!("Received BLE payload: {};", text);
            }

            let packet = decoder.decode_packet(bytearray);
            if packet.is_err() {
                let message: String = packet.unwrap_err();
                println!("Error: {message}");
                continue;
            }
            let packet = packet.unwrap();
            plotter.insert(&packet.samples);

            let mut new_calib_message = None::<String>;
            let mut new_calib_timer = None::<String>;

            // Create a sub-scope because we must drop the MutexGuard before await
            {
                let calibstate = calibrationstate.lock().unwrap();
                if *calibstate {
                    let mut calib = calibrator.lock().unwrap();
                    let state_changed = calib.tick(0.2);
                    if state_changed {
                        new_calib_message = Some(calib.generate_message());
                    }
                    if calib.timer > 0.0 {
                        new_calib_timer = Some(format!("{:.1}s", calib.timer));
                    }
                    else {
                        new_calib_timer = Some(String::new());
                    }
                }
            }

            // Create a sub-scope because we must drop the MutexGuard before await
            {
                let cloned_plotter = plotter.clone();
                let keystate_clone_reader = Arc::clone(&keystate);
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    if let Some(msg) = new_calib_message {
                        ui.set_text_calibration_instruction(msg.into());
                    }
                    if let Some(msg) = new_calib_timer {
                        ui.set_text_calibration_timer(msg.into());
                    }
                    ui.set_graph0(cloned_plotter.render());
                    let keys = keystate_clone_reader.lock().unwrap();
                    let mut keyvec: Vec<&String> = keys.iter().collect();
                    keyvec.sort();
                    ui.set_pressedkeys(
                        keyvec
                            .into_iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<&str>>()
                            .join("")
                            .into(),
                    );
                });
            }
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.05)).await;
        }
    });
    ui.run().unwrap();
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
                    self.remaining_key_presses[self.current_action] = self.remaining_key_presses[self.current_action].saturating_sub(1);
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

#[derive(Clone)]
pub struct Plotter {
    pub data: Vec<VecDeque<f64>>,
}

impl Plotter {
    pub fn new(channel_count: usize) -> Self {
        let data = (0..channel_count)
            .map(|_| VecDeque::with_capacity(MAX_POINTS))
            .collect();
        Self { data }
    }

    pub fn insert(&mut self, items: &Vec<Vec<u8>>) {
        for (channel_index, samples) in items.iter().enumerate() {
            let channel = &mut self.data[channel_index];
            for signal in samples {
                if channel.len() >= MAX_POINTS {
                    channel.pop_front();
                }
                let normalized_signal = ((*signal as f64) - 127.0) / 127.0;
                channel.push_back(normalized_signal);
            }
        }
    }

    pub fn render(&self) -> slint::Image {
        let mut pixel_buffer = SharedPixelBuffer::new(512, 386);
        let size = (pixel_buffer.width(), pixel_buffer.height());
        let backend = BitMapBackend::with_buffer(pixel_buffer.make_mut_bytes(), size);
        let root = backend.into_drawing_area();
        root.fill(&WHITE).expect("error filling drawing area");

        let mut chart = ChartBuilder::on(&root)
            .build_cartesian_2d(0..MAX_POINTS, -8.0..1.0)
            .expect("error building coordinate system");

        chart.configure_mesh().draw().expect("error drawing");

        for (channel, samples) in self.data.iter().enumerate() {
            chart
                .draw_series(LineSeries::new(
                    samples
                        .iter()
                        .enumerate()
                        .map(|(i, x)| (i, *x as f64 - 1.0 * channel as f64)),
                    &match channel % 4 {
                        0 => RED,
                        1 => CYAN,
                        2 => MAGENTA,
                        _ => GREEN,
                    },
                ))
                .expect("error drawing series");
        }

        root.present().expect("error presenting");
        drop(chart);
        drop(root);

        slint::Image::from_rgb8(pixel_buffer)
    }
}
