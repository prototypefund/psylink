use crate::prelude::*;
use plotters::prelude::*;
use slint::SharedPixelBuffer;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
slint::include_modules!();

const MAX_POINTS: usize = 2048;
const EMG_CHANNELS: i32 = 8;
const TOTAL_CHANNELS: usize = 14;

pub async fn start(app: App) {
    let ui = MainWindow::new().unwrap();

    let calib = Arc::new(Mutex::new(calibration::CalibController::default()));
    let calibration_flow = Arc::new(Mutex::new(CalibrationFlow::default()));

    // At the moment, we store the set of keys that are currently being pressed
    // for the purpose of matching them with PsyLink signals in an upcoming feature.
    // If this feature is never added, we can safely throw out keystate.
    let keystate = Arc::new(Mutex::new(HashSet::<String>::new()));
    let keystate_clone_writer = Arc::clone(&keystate);

    let ui_weak = ui.as_weak();
    ui.global::<Logic>()
        .on_key_handler(move |key: slint::SharedString, pressed: bool| {
            let mut keystate = keystate_clone_writer.lock().unwrap();
            let key = key.to_string();
            if pressed {
                if key == "1" || key == "2" {
                    let page = key.parse::<i32>().unwrap() - 1;
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_page(page);
                    });
                } else {
                    keystate.insert(key);
                }
            } else {
                keystate.remove(&key);
            }
        });

    let ui_weak = ui.as_weak();
    let calibration_flow_clone = Arc::clone(&calibration_flow);
    ui.global::<Logic>()
        .on_start_calibration_handler(move |actions: i32| {
            let mut calibration_flow = calibration_flow_clone.lock().unwrap();
            calibration_flow.start(actions as usize, 3);
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_calibrating(true);
                ui.set_text_calibration_instruction(format!("Attempting to calibrate...").into());
            });
        });

    let ui_weak = ui.as_weak();
    let calibration_flow_clone = Arc::clone(&calibration_flow);
    ui.global::<Logic>().on_stop_calibration_handler(move || {
        let mut calibration_flow = calibration_flow_clone.lock().unwrap();
        calibration_flow.stop();
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
        let mut decoder = protocol::Decoder::new(EMG_CHANNELS);
        let mut plotter = Plotter::new(TOTAL_CHANNELS);

        loop {
            // Receive PsyLink signal packet
            let bytearray: Vec<u8> = device.read().await.unwrap(); // TODO: catch panic
            if appclone.verbose > 1 {
                let text = bytearray
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                println!("Received BLE payload: {text};");
            }

            // Decode packet
            let packet = decoder.decode_packet(bytearray);
            if packet.is_err() {
                let message: String = packet.unwrap_err();
                println!("Error: {message}");
                continue;
            }
            let packet = packet.unwrap();

            // Add packet to plotter
            plotter.insert(&packet.samples);

            let mut new_calib_message = None::<String>;
            let mut new_calib_timer = None::<String>;

            // Create a sub-scope because we must drop the MutexGuard before await
            {
                let mut calib_flow = calibration_flow.lock().unwrap();
                let mut calib = calib.lock().unwrap();
                if calib_flow.currently_calibrating {
                    // Update calibration flow state
                    let state_changed = calib_flow.tick(0.2);
                    if state_changed {
                        new_calib_message = Some(calib_flow.generate_message());
                    }
                    if calib_flow.timer > 0.0 {
                        new_calib_timer = Some(format!("{:.1}s", calib_flow.timer));
                    } else {
                        new_calib_timer = Some(String::new());
                    }

                    // Add samples to dataset
                    if let Some(label) = calib_flow.get_label() {
                        for sample in packet.samples {
                            if appclone.verbose > 1 {
                                println!("Adding packet {sample:?}");
                            }
                            calib.add_packet(sample);
                            let datapoint = calibration::Datapoint {
                                packet_index: calib.get_current_index(),
                                label
                            };
                            if appclone.verbose > 1 {
                                println!("Adding datapoint {datapoint:?}");
                            }
                            calib.add_datapoint(datapoint);
                        }
                    }
                }
            }

            // Create a sub-scope because we must drop the MutexGuard before await
            {
                let plotter_clone = plotter.clone();
                let keystate_clone = Arc::clone(&keystate);
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    // Update displayed text
                    if let Some(msg) = new_calib_message {
                        ui.set_text_calibration_instruction(msg.into());
                    }
                    if let Some(msg) = new_calib_timer {
                        ui.set_text_calibration_timer(msg.into());
                    }

                    // Update plotter
                    ui.set_graph0(plotter_clone.render());

                    // Update display of currently pressed keys
                    let keys = keystate_clone.lock().unwrap();
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

        let x_axis = 0..MAX_POINTS;
        let y_axis = -(TOTAL_CHANNELS as f64 + 1.0)..1.0;
        let mut chart = ChartBuilder::on(&root)
            .build_cartesian_2d(x_axis, y_axis)
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

#[derive(Clone, Default)]
pub struct CalibrationFlow {
    pub currently_calibrating: bool,
    pub action_count: usize,
    pub current_action: usize,
    pub remaining_key_presses: Vec<u32>,
    pub timer: f64,
    pub state: CalibrationFlowState,
}

#[derive(Clone, Default, PartialEq)]
pub enum CalibrationFlowState {
    #[default]
    Init,
    Welcome,
    NullActionWait,
    NullAction,
    GestureActionWait,
    GestureAction,
    Done,
}

impl CalibrationFlow {
    pub fn start(&mut self, action_count: usize, key_presses: u32) {
        self.action_count = action_count;
        for _ in 0..action_count {
            self.remaining_key_presses.push(key_presses);
        }
        self.currently_calibrating = true;
    }

    pub fn stop(&mut self) {
        *self = Self::default();
    }

    pub fn get_label(&self) -> Option<u8> {
        if !self.currently_calibrating {
            return None;
        }
        match self.state {
            // NOTE: THIS MAY PANIC:
            CalibrationFlowState::GestureAction => Some(self.current_action as u8 + 1),
            CalibrationFlowState::NullAction => Some(0),
            _ => None,
        }
    }

    /// returns true if a state change happened
    pub fn tick(&mut self, time: f64) -> bool {
        if self.timer > 0.0 {
            self.timer -= time;
        }
        if self.timer <= 0.0 {
            let new_state = match self.state {
                CalibrationFlowState::Init => CalibrationFlowState::Welcome,
                CalibrationFlowState::Welcome => CalibrationFlowState::NullActionWait,
                CalibrationFlowState::NullActionWait => CalibrationFlowState::NullAction,
                CalibrationFlowState::NullAction => {
                    if self.remaining_key_presses.iter().all(|&x| x <= 0) {
                        CalibrationFlowState::Done
                    } else {
                        CalibrationFlowState::GestureActionWait
                    }
                }
                CalibrationFlowState::GestureActionWait => CalibrationFlowState::GestureAction,
                CalibrationFlowState::GestureAction => {
                    self.remaining_key_presses[self.current_action] =
                        self.remaining_key_presses[self.current_action].saturating_sub(1);
                    self.current_action = (self.current_action + 1) % self.action_count;

                    CalibrationFlowState::NullActionWait
                }
                CalibrationFlowState::Done => CalibrationFlowState::Done,
            };
            let delay = match new_state {
                CalibrationFlowState::Init | CalibrationFlowState::Done => 0.0,
                CalibrationFlowState::Welcome => 4.0,
                CalibrationFlowState::NullActionWait => 3.0,
                CalibrationFlowState::NullAction => 8.0,
                CalibrationFlowState::GestureActionWait => 4.0,
                CalibrationFlowState::GestureAction => 8.0,
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
            CalibrationFlowState::Init => "Initializing...".into(),
            CalibrationFlowState::Welcome => {
                "Calibration starting. Please follow the instructions.".into()
            }
            CalibrationFlowState::NullActionWait => "Prepare to rest your arm.".into(),
            CalibrationFlowState::NullAction => "Rest your arm now.".into(),
            CalibrationFlowState::GestureActionWait => {
                format!("Prepare movement #{current_action}")
            }
            CalibrationFlowState::GestureAction => format!("Do movement #{current_action} now."),
            CalibrationFlowState::Done => "Calibration complete.".into(),
        }
    }
}
