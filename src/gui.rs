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
    let state = GUIState::new();

    let ui = MainWindow::new().unwrap();
    ui.set_train_max_datapoints(slint::SharedString::from(
        state.train_max_datapoints.to_string(),
    ));
    ui.set_train_epochs(slint::SharedString::from(state.train_epochs.to_string()));

    // Naming convention:
    // orig_mutex_ABC = original Arc<Mutex<...>> struct
    // mutex_ABC = cloned mutex_ABC for bringing mutex_ABC into thread scope
    // ABC = cloned_ABC.lock().unwrap() inside thread, when ABC needs to be read/written
    let orig_mutex_calib = Arc::new(Mutex::new(calibration::CalibController::default()));
    let orig_mutex_flow = Arc::new(Mutex::new(CalibrationFlow::default()));
    let orig_mutex_settings = Arc::new(Mutex::new(GUISettings::new()));
    let orig_mutex_model = Arc::new(Mutex::new(None::<calibration::DefaultModel>));
    let orig_mutex_commands = Arc::new(Mutex::new(GUICommands::default()));
    let orig_mutex_state = Arc::new(Mutex::new(state));
    let orig_mutex_plotter = Arc::new(Mutex::new(Plotter::new(TOTAL_CHANNELS)));
    let orig_mutex_quit = Arc::new(Mutex::new(false));
    let orig_mutex_fakeinput = Arc::new(Mutex::new(fakeinput::InputState::new(app.verbose > 0)));

    // At the moment, we store the set of keys that are currently being pressed
    // for the purpose of matching them with PsyLink signals in an upcoming feature.
    // If this feature is never added, we can safely throw out keystate.
    let orig_mutex_keystate = Arc::new(Mutex::new(HashSet::<String>::new()));

    let ui_weak = ui.as_weak();
    let mutex_keystate = orig_mutex_keystate.clone();
    ui.global::<Logic>()
        .on_key_handler(move |key: slint::SharedString, pressed: bool| {
            let mut keystate = mutex_keystate.lock().unwrap();
            let key = key.to_string();
            if pressed {
                if key == "1" || key == "2" || key == "3" {
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
    let mutex_flow = orig_mutex_flow.clone();
    let mutex_calib = orig_mutex_calib.clone();
    let mutex_settings = orig_mutex_settings.clone();
    ui.global::<Logic>().on_start_calibration_handler(move || {
        let action_count = mutex_settings.lock().unwrap().action_count;
        mutex_flow.lock().unwrap().start(action_count, 2);
        mutex_calib.lock().unwrap().reset();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_calibrating(true);
            ui.set_text_calibration_instruction(format!("Attempting to calibrate...").into());
        });
    });

    let ui_weak = ui.as_weak();
    let mutex_flow = orig_mutex_flow.clone();
    ui.global::<Logic>().on_stop_calibration_handler(move || {
        mutex_flow.lock().unwrap().stop();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_calibrating(false);
            ui.set_text_calibration_instruction(format!("No calibration in progress.").into());
        });
    });

    let mutex_settings = orig_mutex_settings.clone();
    ui.global::<Logic>().on_set_option_action_count(
        move |action_count_string: slint::SharedString| {
            let first_char = action_count_string.chars().next().unwrap();
            let action_count = first_char.to_string().parse::<usize>().unwrap();
            mutex_settings.lock().unwrap().action_count = action_count as usize;
        },
    );

    let mutex_state = orig_mutex_state.clone();
    ui.global::<Logic>()
        .on_set_option_epochs(move |value: slint::SharedString| {
            let parsed = value.to_string().parse::<usize>().unwrap();
            mutex_state.lock().unwrap().train_epochs = parsed;
        });

    let mutex_state = orig_mutex_state.clone();
    ui.global::<Logic>()
        .on_set_option_max_datapoints(move |value: slint::SharedString| {
            let parsed = value.to_string().parse::<usize>().unwrap();
            mutex_state.lock().unwrap().train_max_datapoints = parsed;
        });

    let mutex_settings = orig_mutex_settings.clone();
    ui.global::<Logic>()
        .on_set_option_accelerometer(move |checked: bool| {
            let mut settings = mutex_settings.lock().unwrap();
            settings.disable_accelerometer = !checked;
        });

    let mutex_settings = orig_mutex_settings.clone();
    ui.global::<Logic>()
        .on_set_option_gyroscope(move |checked: bool| {
            let mut settings = mutex_settings.lock().unwrap();
            settings.disable_gyroscope = !checked;
        });

    let mutex_fakeinput = orig_mutex_fakeinput.clone();
    ui.global::<Logic>().on_set_option_keypress_value(
        move |action_id: i32, chosen_text: slint::SharedString| {
            let action = match chosen_text.as_str() {
                "Nothing" => Action::None,
                "Space" => Action::Key(' '),
                "Sound" => Action::Sound(440.0),
                _ => Action::Key(chosen_text.chars().next().unwrap()),
            };
            let mut fakeinput = mutex_fakeinput.lock().unwrap();
            fakeinput.set_action(action_id as usize, action);
        },
    );

    let ui_weak = ui.as_weak();
    let mutex_state = orig_mutex_state.clone();
    let mutex_settings = orig_mutex_settings.clone();
    ui.global::<Logic>().on_train_handler(move || {
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_training(true);
            ui.set_text_calibration_instruction("Training...".into());
        });
        mutex_state.lock().unwrap().training = true;
    });

    let mutex_calib = orig_mutex_calib.clone();
    let mutex_state = orig_mutex_state.clone();
    ui.global::<Logic>().on_save_dataset_handler(move || {
        let path = "/tmp/psylink_dataset.rs";
        let calib = mutex_calib.lock().unwrap();
        let mut output = std::fs::File::create(path).unwrap();
        let _ = write!(output, "{}", calib.dataset.to_string());
        mutex_state
            .lock()
            .unwrap()
            .log(format!("Saved dataset to {path}."));
    });

    let mutex_state = orig_mutex_state.clone();
    ui.global::<Logic>().on_save_log_handler(move || {
        let path = "/tmp/psylink_log.txt";
        let mut output = std::fs::File::create(path).unwrap();
        if let Ok(mut state) = mutex_state.lock() {
            let _ = write!(output, "{}", state.log2string());
            state.log(format!("Saved log to {path}."));
        }
    });

    let mutex_calib = orig_mutex_calib.clone();
    let mutex_state = orig_mutex_state.clone();
    ui.global::<Logic>().on_load_dataset_handler(move || {
        mutex_state.lock().unwrap().update_statusbar = true;
        mutex_calib.lock().unwrap().dataset =
            PsyLinkDataset::from_arrays(&TEST_DATASET.0, &TEST_DATASET.1);
    });

    let ui_weak = ui.as_weak();
    let mutex_model = orig_mutex_model.clone();
    let mutex_state = orig_mutex_state.clone();
    ui.global::<Logic>().on_load_model_handler(move || {
        let mut model = mutex_model.lock().unwrap();
        if let Ok(mut state) = mutex_state.lock() {
            state.update_statusbar = true;
            state.trained = true;
        }
        *model = Some(calibration::load_test_model());
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_model_trained(true);
        });
    });

    let mutex_flow = orig_mutex_flow.clone();
    let ui_weak = ui.as_weak();
    let mutex_model = orig_mutex_model.clone();
    ui.global::<Logic>().on_infer_start_handler(move || {
        let model = mutex_model.lock().unwrap();
        if (*model).is_some() {
            let mut calibration_flow = mutex_flow.lock().unwrap();
            calibration_flow.currently_inferring = true;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_inferring(true);
            });
        }
    });

    let mutex_flow = orig_mutex_flow.clone();
    let ui_weak = ui.as_weak();
    ui.global::<Logic>().on_infer_stop_handler(move || {
        let mut calibration_flow = mutex_flow.lock().unwrap();
        calibration_flow.currently_inferring = false;
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_inferring(false);
        });
    });

    // The thread for inference/prediction
    let mutex_quit = orig_mutex_quit.clone();
    let mutex_flow = orig_mutex_flow.clone();
    let mutex_model = orig_mutex_model.clone();
    let mutex_state = orig_mutex_state.clone();
    let mutex_calib = orig_mutex_calib.clone();
    let mutex_commands = orig_mutex_commands.clone();
    let mutex_fakeinput = orig_mutex_fakeinput.clone();
    let ui_weak = ui.as_weak();
    let appclone = app.clone();
    tokio::spawn(async move {
        loop {
            let currently_inferring: bool = {
                // Create a sub-scope to drop the MutexGuard afterwards
                let calib_flow = mutex_flow.lock().unwrap();
                calib_flow.currently_inferring
            };
            if currently_inferring {
                let model = mutex_model.lock().unwrap();
                let calib = mutex_calib.lock().unwrap();
                if (*model).is_some() {
                    let inferred = calib.infer_latest((*model).clone().unwrap());
                    if let Some(key) = inferred {
                        {
                            let mut gui_commands = mutex_commands.lock().unwrap();
                            gui_commands.change_predicted_key = Some(key.to_string());
                        }
                        {
                            let mut fakeinput = mutex_fakeinput.lock().unwrap();
                            fakeinput.set_predicted(key as u8);
                        }
                    }
                } else {
                    mutex_flow.lock().unwrap().currently_inferring = false;
                    println!("WARNING: attempted to infer before model is loaded");
                }
            }
            if mutex_state.lock().unwrap().training {
                let (epochs, max_datapoints) = if let Ok(mut state) = mutex_state.lock() {
                    state.training = false;
                    (state.train_epochs, state.train_max_datapoints)
                } else {
                    (
                        calibration::DEFAULT_EPOCHS,
                        calibration::DEFAULT_MAX_DATAPOINTS,
                    )
                };
                let calib = mutex_calib.lock().unwrap();
                let action_count = mutex_settings.lock().unwrap().action_count;
                let result = calib.train(action_count, epochs, max_datapoints);
                dbg!(&result);
                if let Ok(trained_model) = result {
                    let mut model = mutex_model.lock().unwrap();
                    let model_log = format!("{:?}", &trained_model);
                    *model = Some(trained_model);
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_training(false);
                        ui.set_model_trained(true);
                        ui.set_text_calibration_instruction("Training complete.".into());
                    });
                    if let Ok(mut state) = mutex_state.lock() {
                        state.log("Finished training AI calibration model.".into());
                        state.log(format!("Training result: {model_log}").into());
                        state.trained = true;
                        state.update_statusbar = true;
                    }
                } else {
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_training(false);
                        ui.set_text_calibration_instruction("Training failed.".into());
                    });
                    mutex_state
                        .lock()
                        .unwrap()
                        .log("Failed training AI calibration model.".into());
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.1)).await;
            if *(mutex_quit.lock().unwrap()) {
                if appclone.verbose > 0 {
                    println!("Quitting inference thread!");
                }
                break;
            }
        }
    });

    // The thread for plotting the signals
    let ui_weak = ui.as_weak();
    let mutex_quit = orig_mutex_quit.clone();
    let mutex_plotter = orig_mutex_plotter.clone();
    let mutex_state = orig_mutex_state.clone();
    tokio::spawn(async move {
        loop {
            if mutex_state.lock().unwrap().connected {
                let plotter = mutex_plotter.lock().unwrap().clone();
                let rendered = plotter.render();
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_graph0(slint::Image::from_rgb8(rendered));
                });
                tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.005)).await;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.2)).await;
            }

            if *(mutex_quit.lock().unwrap()) {
                if appclone.verbose > 0 {
                    println!("Quitting plotter thread!");
                }
                break;
            }
        }
    });

    // The thread for receiving and storing packages
    let ui_weak = ui.as_weak();
    let appclone = app.clone();
    let mutex_quit = orig_mutex_quit.clone();
    let mutex_calib = orig_mutex_calib.clone();
    let mutex_flow = orig_mutex_flow.clone();
    let mutex_plotter = orig_mutex_plotter.clone();
    let mutex_commands = orig_mutex_commands.clone();
    let mutex_settings = orig_mutex_settings.clone();
    let mutex_state = orig_mutex_state.clone();
    let thread_network = tokio::spawn(async move {
        let mut device = loop {
            mutex_state.lock().unwrap().update_statusbar = true;
            if let Ok(device) = bluetooth::find_peripheral(appclone, Some(mutex_quit.clone())).await
            {
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
            if *(mutex_quit.lock().unwrap()) {
                if appclone.verbose > 0 {
                    println!("Quitting networking thread!");
                }
                return;
            }
        };
        device.find_characteristics().await;
        {
            // Create a sub-scope to drop the MutexGuard afterwards
            let mut state = mutex_state.lock().unwrap();
            state.connected = true;
            state.log(format!(
                "Connected to PsyLink with MAC address {}.",
                device.address.clone()
            ));
            state.update_statusbar = true;
        }

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
                let settings = mutex_settings.lock().unwrap();
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
                let mut plotter = mutex_plotter.lock().unwrap();
                plotter.insert(&packet.samples);
            }

            // Create a sub-scope because we must drop the MutexGuard before await
            {
                let mut calib_flow = mutex_flow.lock().unwrap();
                let mut calib = mutex_calib.lock().unwrap();
                if calib_flow.currently_calibrating || calib_flow.currently_inferring {
                    if calib_flow.currently_calibrating {
                        // Update calibration flow state
                        let state_changed = calib_flow.tick(dt);
                        {
                            let mut gui_commands = mutex_commands.lock().unwrap();
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
                    mutex_state.lock().unwrap().update_statusbar = true;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.001)).await;
            if *(mutex_quit.lock().unwrap()) {
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
    let ui_weak = ui.as_weak();
    let mutex_commands = orig_mutex_commands.clone();
    let mutex_keystate = orig_mutex_keystate.clone();
    let mutex_quit = orig_mutex_quit.clone();
    tokio::spawn(async move {
        loop {
            let mut gui_commands = mutex_commands.lock().unwrap().clone();
            let keystate = mutex_keystate.lock().unwrap().clone();
            let mutex_state = orig_mutex_state.clone();
            let mutex_model = orig_mutex_model.clone();
            let mutex_calib = orig_mutex_calib.clone();

            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                // Update displayed text
                if let Some(msg) = gui_commands.change_calib_message {
                    ui.set_text_calibration_instruction(msg.into());
                    gui_commands.change_calib_message = None;
                }
                if let Some(msg) = gui_commands.change_calib_timer {
                    ui.set_text_calibration_timer(msg.into());
                    gui_commands.change_calib_timer = None;
                }

                if let Some(msg) = gui_commands.change_predicted_key {
                    ui.set_text_predicted(msg.into());
                    gui_commands.change_predicted_key = None;
                }

                ui.set_sampled(mutex_calib.lock().unwrap().has_datapoints());

                if let Ok(mut state) = mutex_state.lock() {
                    if state.update_log {
                        ui.set_log(state.log2string().into());
                        state.update_log = false;
                    }

                    if state.update_statusbar {
                        let con = if state.connected { "Yes" } else { "No" };
                        let cal = if mutex_model.lock().unwrap().is_some() {
                            "Yes"
                        } else {
                            "No"
                        };
                        let (sampl, dpts) = {
                            let calib = mutex_calib.lock().unwrap();
                            (calib.get_current_index(), calib.count_datapoints())
                        };
                        ui.set_text_statusbar(
                            format!("Connected: {con}, Calibrated: {cal}, Samples: {sampl}, Training datapoints: {dpts}").into(),
                        );
                        state.update_statusbar = false;
                    }
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

            if *(mutex_quit.lock().unwrap()) {
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
        let mut do_quit = orig_mutex_quit.lock().unwrap();
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
pub struct GUICommands {
    pub change_calib_message: Option<String>,
    pub change_calib_timer: Option<String>,
    pub change_predicted_key: Option<String>,
}

#[derive(Clone, Default)]
pub struct GUISettings {
    pub disable_gyroscope: bool,
    pub disable_accelerometer: bool,
    pub action_count: usize,
}

impl GUISettings {
    pub fn new() -> Self {
        let mut result = Self::default();
        result.action_count = 1;
        result
    }
}

#[derive(Clone, Default)]
pub struct GUIState {
    pub connected: bool,
    pub sampled: bool,
    pub trained: bool,
    pub training: bool,
    pub log_entries: Vec<String>,
    pub update_statusbar: bool,
    pub update_log: bool,
    pub train_max_datapoints: usize,
    pub train_epochs: usize,
}

impl GUIState {
    pub fn new() -> Self {
        let mut result = Self::default();
        result.train_max_datapoints = calibration::DEFAULT_MAX_DATAPOINTS;
        result.train_epochs = calibration::DEFAULT_EPOCHS;
        result
    }

    pub fn log(&mut self, entry: String) {
        self.log_entries.push(entry);
        self.update_log = true;
    }

    pub fn log2string(&self) -> String {
        self.log_entries.join("\n")
    }
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
        self.state = CalibrationFlowState::Init;
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
            CalibrationFlowState::Welcome => "Please follow the instructions.".into(),
            CalibrationFlowState::NullActionWait => "⚠️ Prepare to rest your arm.".into(),
            CalibrationFlowState::NullAction => "⛔ Rest your arm now.".into(),
            CalibrationFlowState::GestureActionWait => {
                format!("⚠️ Prepare movement #{current_action}")
            }
            CalibrationFlowState::GestureAction => format!("✋ Do movement #{current_action} now."),
            CalibrationFlowState::Done => "Data collected. Click 'Train AI'.".into(),
        }
    }
}
