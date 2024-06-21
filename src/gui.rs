use crate::{base, bluetooth, protocol};
use plotters::prelude::*;
use slint::SharedPixelBuffer;
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, VecDeque};
slint::include_modules!();

const MAX_POINTS: usize = 2048;

pub async fn start(app: base::App) {
    let ui = MainWindow::new().unwrap();
    let ui_weak = ui.as_weak();

    let keystate = Arc::new(Mutex::new(HashSet::<String>::new()));
    let keystate_clone_writer = Arc::clone(&keystate);
    let keystate_clone_reader = Arc::clone(&keystate);
    ui.global::<Logic>().on_key_handler(move |key: slint::SharedString, pressed: bool| {
        let mut keystate = keystate_clone_writer.lock().unwrap();
        if pressed {
            keystate.insert(key.to_string());
        } else {
            keystate.remove(&key.to_string());
        }
    });

    let appclone = app.clone();
    tokio::spawn(async move {
        let mut device = loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            if let Ok(device) = bluetooth::find_peripheral(appclone).await {
                let address = device.address.clone();
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_statustext(
                        format!("Found PsyLink with MAC address {address}.\n\nConnecting...")
                            .into(),
                    );
                });
                break device;
            }
        };
        device.find_characteristics().await;

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_statustext(format!("Displaying PsyLink signals.").into());
            ui.set_page(1);
        });
        let mut decoder = protocol::Decoder::new(8);
        let mut plotter = Plotter::new(8);

        loop {
            dbg!(&keystate_clone_reader);
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

            let cloned_plotter = plotter.clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_graph0(cloned_plotter.render());
            });
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.1)).await;
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
        let mut pixel_buffer = SharedPixelBuffer::new(800, 600);
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
