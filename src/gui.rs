use crate::calibration::{PsyLinkDataset, TEST_DATASET};
use crate::prelude::*;
use plotters::prelude::*;
use slint::SharedPixelBuffer;
use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
slint::include_modules!();

const MAX_POINTS: usize = 2000;
const EMG_CHANNELS: i32 = 8;
const TOTAL_CHANNELS: usize = 14;

const BG_COLOR: RGBColor = RGBColor(0x1c, 0x1c, 0x1c);

pub async fn start(app: App) {
    let ui = MainWindow::new().unwrap();

    let calib = Arc::new(Mutex::new(calibration::CalibController::default()));
    let calibration_flow = Arc::new(Mutex::new(CalibrationFlow::default()));
    let settings = Arc::new(Mutex::new(GUISettings::default()));
    let model = Arc::new(Mutex::new(None::<calibration::DefaultModel>));
    let gui_commands = Arc::new(Mutex::new(GuiCommands::default()));
    let state = Arc::new(Mutex::new(GUIState::default()));
    let plotter = Arc::new(Mutex::new(Plotter::new(TOTAL_CHANNELS)));
    let do_quit = Arc::new(Mutex::new(false));
    let fakeinput = Arc::new(Mutex::new(fakeinput::InputState::new(app.verbose > 0)));

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
    let calib_clone = Arc::clone(&calib);
    ui.global::<Logic>()
        .on_start_calibration_handler(move |actions: i32| {
            let mut calibration_flow = calibration_flow_clone.lock().unwrap();
            let mut calib = calib_clone.lock().unwrap();
            calibration_flow.start(actions as usize, 2);
            calib.reset();
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

    let settings_clone = Arc::clone(&settings);
    ui.global::<Logic>()
        .on_set_option_accelerometer(move |checked: bool| {
            let mut settings = settings_clone.lock().unwrap();
            settings.disable_accelerometer = !checked;
        });

    let settings_clone = Arc::clone(&settings);
    ui.global::<Logic>()
        .on_set_option_gyroscope(move |checked: bool| {
            let mut settings = settings_clone.lock().unwrap();
            settings.disable_gyroscope = !checked;
        });

    let fakeinput_clone = fakeinput.clone();
    ui.global::<Logic>()
        .on_set_option_keypress_value(move |chosen_text: slint::SharedString| {
            let action = match chosen_text.as_str() {
                "Nothing" => None,
                "Space" => Some(' '),
                _ => Some(chosen_text.chars().next().unwrap()),
            };
            let mut fakeinput = fakeinput_clone.lock().unwrap();
            fakeinput.set_action(1, action);
        });

    let calib_clone = Arc::clone(&calib);
    let model_clone = Arc::clone(&model);
    let ui_weak = ui.as_weak();
    ui.global::<Logic>().on_train_handler(move || {
        let calib = calib_clone.lock().unwrap();
        let result = calib.train();
        dbg!(&result);
        if let Ok(trained_model) = result {
            let mut model = model_clone.lock().unwrap();
            *model = Some(trained_model);
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_model_trained(true);
            });
        }
    });

    let calib_clone = Arc::clone(&calib);
    ui.global::<Logic>().on_save_handler(move || {
        let calib = calib_clone.lock().unwrap();
        let mut output = std::fs::File::create("/tmp/saved_psylink_dataset.rs").unwrap();
        let _ = write!(output, "{}", calib.dataset.to_string());
    });

    let calib_clone = Arc::clone(&calib);
    ui.global::<Logic>().on_load_handler(move || {
        let mut calib = calib_clone.lock().unwrap();
        calib.dataset = PsyLinkDataset::from_arrays(&TEST_DATASET.0, &TEST_DATASET.1);
    });

    let model_clone = Arc::clone(&model);
    let ui_weak = ui.as_weak();
    ui.global::<Logic>().on_load_model_handler(move || {
        let mut model = model_clone.lock().unwrap();
        *model = Some(calibration::load_test_model());
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_model_trained(true);
        });
    });

    let calibration_flow_clone = Arc::clone(&calibration_flow);
    let ui_weak = ui.as_weak();
    let model_clone = Arc::clone(&model);
    ui.global::<Logic>().on_infer_start_handler(move || {
        let model = model_clone.lock().unwrap();
        if (*model).is_some() {
            let mut calibration_flow = calibration_flow_clone.lock().unwrap();
            calibration_flow.currently_inferring = true;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_inferring(true);
            });
        }
    });

    let calibration_flow_clone = Arc::clone(&calibration_flow);
    let ui_weak = ui.as_weak();
    ui.global::<Logic>().on_infer_stop_handler(move || {
        let mut calibration_flow = calibration_flow_clone.lock().unwrap();
        calibration_flow.currently_inferring = false;
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_inferring(false);
        });
    });

    // The thread for inference/prediction
    let do_quit_clone = do_quit.clone();
    let calibration_flow_clone = Arc::clone(&calibration_flow);
    let model_clone = Arc::clone(&model);
    let calib_clone = Arc::clone(&calib);
    let gui_commands_clone = gui_commands.clone();
    let fakeinput_clone = fakeinput.clone();
    let appclone = app.clone();
    tokio::spawn(async move {
        loop {
            let currently_inferring: bool = {
                // Create a sub-scope to drop the MutexGuard afterwards
                let calib_flow = calibration_flow_clone.lock().unwrap();
                calib_flow.currently_inferring
            };
            if currently_inferring {
                let model = model_clone.lock().unwrap();
                let calib = calib_clone.lock().unwrap();
                if (*model).is_some() {
                    let inferred = calib.infer_latest((*model).clone().unwrap());
                    if let Some(key) = inferred {
                        {
                            let mut gui_commands = gui_commands_clone.lock().unwrap();
                            gui_commands.change_predicted_key = Some(key.to_string());
                        }
                        {
                            let mut fakeinput = fakeinput_clone.lock().unwrap();
                            fakeinput.set_predicted(key as u8);
                        }
                    }
                } else {
                    // Create a sub-scope to drop the MutexGuard afterwards
                    {
                        let mut calib_flow = calibration_flow_clone.lock().unwrap();
                        calib_flow.currently_inferring = false;
                    }
                    println!("WARNING: attempted to infer before model is loaded");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.1)).await;
            if *(do_quit_clone.lock().unwrap()) {
                if appclone.verbose > 0 {
                    println!("Quitting inference thread!");
                }
                break;
            }
        }
    });

    // The thread for plotting the signals
    let do_quit_clone = do_quit.clone();
    let plotter_clone = plotter.clone();
    let state_clone = state.clone();
    let ui_weak = ui.as_weak();
    tokio::spawn(async move {
        loop {
            if state_clone.lock().unwrap().connected {
                let plotter = plotter_clone.lock().unwrap().clone();
                let rendered = plotter.render();
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_graph0(slint::Image::from_rgb8(rendered));
                });
                tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.005)).await;
            }
            else {
                tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.2)).await;
            }

            if *(do_quit_clone.lock().unwrap()) {
                if appclone.verbose > 0 {
                    println!("Quitting plotter thread!");
                }
                break;
            }
        }
    });

    // The thread for receiving and storing packages
    let do_quit_clone = do_quit.clone();
    let appclone = app.clone();
    let plotter_clone = plotter.clone();
    let gui_commands_clone = gui_commands.clone();
    let settings_clone = Arc::clone(&settings);
    let state_clone = state.clone();
    let ui_weak = ui.as_weak();
    let thread_network = tokio::spawn(async move {
        let mut device = loop {
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
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.25)).await;
        };
        device.find_characteristics().await;
        state_clone.lock().unwrap().connected = true;

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_connected(true);
            ui.set_text_connection_title(
                format!("PsyLink connection established.\n\nPlease select another tab.").into(),
            );
            ui.set_text_graph_title(format!("Displaying PsyLink signals.").into());
            ui.set_page(1);
        });
        let mut decoder = protocol::Decoder::new(EMG_CHANNELS);

        let mut time = SystemTime::now();

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
            let (enable_accelerometer, enable_gyroscope) = {
                let settings = settings_clone.lock().unwrap();
                (!settings.disable_accelerometer, !settings.disable_gyroscope)
            };
            let packet = decoder.decode_packet(bytearray, enable_accelerometer, enable_gyroscope);
            if packet.is_err() {
                let message: String = packet.unwrap_err();
                println!("Error: {message}");
                continue;
            }
            let packet = packet.unwrap();
            if packet.is_duplicate {
                if appclone.verbose > 0 {
                    println!("Dropping duplicate packet.");
                }
                continue;
            }
            if packet.lost_packets > 0 {
                println!(
                    "Warning: lost {} packet{}",
                    packet.lost_packets,
                    if packet.lost_packets == 1 { "" } else { "s" }
                );
            }

            let dt = if let Ok(duration) = time.elapsed() {
                duration.as_secs_f64()
            } else {
                0.1
            };
            time = SystemTime::now();

            // Add packet to plotter
            {
                let mut plotter = plotter_clone.lock().unwrap();
                plotter.insert(&packet.samples);
            }

            // Create a sub-scope because we must drop the MutexGuard before await
            {
                let mut calib_flow = calibration_flow.lock().unwrap();
                let mut calib = calib.lock().unwrap();
                if calib_flow.currently_calibrating || calib_flow.currently_inferring {
                    if calib_flow.currently_calibrating {
                        // Update calibration flow state
                        let state_changed = calib_flow.tick(dt);
                        {
                            let mut gui_commands = gui_commands_clone.lock().unwrap();
                            if state_changed {
                                gui_commands.change_calib_message =
                                    Some(calib_flow.generate_message());
                                match calib_flow.state {
                                    CalibrationFlowState::Done => {
                                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                            ui.set_calibrating(false);
                                        });
                                    }
                                    _ => {}
                                }
                            }
                            if calib_flow.timer > 0.0 {
                                gui_commands.change_calib_timer =
                                    Some(format!("{:.1}s", calib_flow.timer));
                            } else {
                                gui_commands.change_calib_timer = Some(String::new());
                            }
                        }
                    }

                    // Add samples to dataset
                    let label_maybe = calib_flow.get_label();
                    for sample in transpose_vec(packet.samples) {
                        // Always add the packet, so we have a history of packets
                        // from which we can construct the training samples
                        if appclone.verbose > 0 {
                            println!("Adding packet {sample:?}");
                        }
                        calib.add_packet(sample);

                        // Add datapoints only if UI asks the user to perform some action
                        if let Some(label) = label_maybe {
                            let datapoint = calibration::Datapoint {
                                packet_index: calib.get_current_index(),
                                label,
                            };
                            if appclone.verbose > 0 {
                                println!("Adding datapoint {datapoint:?}");
                            }
                            calib.add_datapoint(datapoint);
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.001)).await;
            if *(do_quit_clone.lock().unwrap()) {
                if appclone.verbose > 0 {
                    println!("Quitting networking thread!");
                }
                println!("Disconnecting...");
                device.disconnect().await.expect("Failed to disconnect");
                break;
            }
        }
    });

    // The thread for updating UI elements
    let gui_commands_clone = gui_commands.clone();
    let keystate_clone = Arc::clone(&keystate);
    let ui_weak = ui.as_weak();
    let do_quit_clone = do_quit.clone();
    tokio::spawn(async move {
        loop {
            let gui_commands = gui_commands_clone.lock().unwrap().clone();
            let keystate = keystate_clone.lock().unwrap().clone();

            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                // Update displayed text
                if let Some(msg) = gui_commands.change_calib_message {
                    ui.set_text_calibration_instruction(msg.into());
                }
                if let Some(msg) = gui_commands.change_calib_timer {
                    ui.set_text_calibration_timer(msg.into());
                }

                if let Some(msg) = gui_commands.change_predicted_key {
                    ui.set_text_predicted(msg.into());
                }

                if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                    ui.set_animation_tick(((now.as_millis() % 1000000) / 100) as i32);
                }

                // Update display of currently pressed keys
                let mut keyvec: Vec<&String> = keystate.iter().collect();
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
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.005)).await;

            if *(do_quit_clone.lock().unwrap()) {
                if appclone.verbose > 0 {
                    println!("Quitting UI update thread!");
                }
                break;
            }
        }
    });

    ui.run().unwrap();

    // Signal threads to terminate themselves
    {
        let mut do_quit = do_quit.lock().unwrap();
        *do_quit = true;
    }
    let _ = tokio::join!(thread_network);
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

    pub fn render(&self) -> SharedPixelBuffer<slint::Rgb8Pixel> {
        let mut pixel_buffer = SharedPixelBuffer::new(512, 386);
        let size = (pixel_buffer.width(), pixel_buffer.height());
        let backend = BitMapBackend::with_buffer(pixel_buffer.make_mut_bytes(), size);
        let root = backend.into_drawing_area();
        root.fill(&BG_COLOR).expect("error filling drawing area");

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

        pixel_buffer
    }
}

/// A thread messaging struct for deferring UI changes to a separate thread
#[derive(Clone, Default)]
pub struct GuiCommands {
    pub change_calib_message: Option<String>,
    pub change_calib_timer: Option<String>,
    pub change_predicted_key: Option<String>,
}

#[derive(Clone, Default)]
pub struct GUISettings {
    pub disable_gyroscope: bool,
    pub disable_accelerometer: bool,
}

#[derive(Clone, Default)]
pub struct GUIState {
    pub connected: bool,
}

#[derive(Clone, Default)]
pub struct CalibrationFlow {
    pub currently_calibrating: bool,
    pub currently_inferring: bool,
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
                CalibrationFlowState::Welcome => 3.0,
                CalibrationFlowState::NullActionWait => 2.5,
                CalibrationFlowState::NullAction => 5.0,
                CalibrationFlowState::GestureActionWait => 2.5,
                CalibrationFlowState::GestureAction => 5.0,
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
                "Please follow the instructions.".into()
            }
            CalibrationFlowState::NullActionWait => "⚠️ Prepare to rest your arm.".into(),
            CalibrationFlowState::NullAction => "⛔ Rest your arm now.".into(),
            CalibrationFlowState::GestureActionWait => {
                format!("⚠️ Prepare movement #{current_action}")
            }
            CalibrationFlowState::GestureAction => format!("✋ Do movement #{current_action} now."),
            CalibrationFlowState::Done => "Calibration complete.".into(),
        }
    }
}
