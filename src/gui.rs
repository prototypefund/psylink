use crate::prelude::*;
use calibration::Calibrator;
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

    let calibrator = Arc::new(Mutex::new(Calibrator::default()));
    let calibrationstate = Arc::new(Mutex::new(false));

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
                }
                else {
                    keystate.insert(key);
                }
            } else {
                keystate.remove(&key);
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
        let mut decoder = protocol::Decoder::new(EMG_CHANNELS);
        let mut plotter = Plotter::new(TOTAL_CHANNELS);

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
                    } else {
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
