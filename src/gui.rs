use crate::{base, bluetooth, protocol};
use plotters::prelude::*;
use slint::SharedPixelBuffer;
slint::include_modules!();

pub async fn start(app: base::App) {
    let ui = MainWindow::new().unwrap();
    let ui_weak = ui.as_weak();

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

        loop {
            let bytearray: Vec<u8> = device.read().await.unwrap(); // TODO: catch panic
            let text = bytearray
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            println!("Received BLE payload: {};", text);

            let packet = decoder.decode_packet(bytearray);
            if packet.is_err() {
                let message: String = packet.unwrap_err();
                println!("Error: {message}");
                continue;
            }
            let packet = packet.unwrap();

            let mut string = String::new();
            for data in &packet.samples {
                string += format!("{data:?}\n").as_str();
            }
            let signal_data = packet.samples.clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_datatext(string.into());
                ui.set_graph0(render_plot(signal_data));
            });
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.1)).await;
        }
    });
    ui.run().unwrap();
}

fn render_plot(signal_data: Vec<Vec<u8>>) -> slint::Image {
    let mut pixel_buffer = SharedPixelBuffer::new(800, 600);
    let size = (pixel_buffer.width(), pixel_buffer.height());
    let backend = BitMapBackend::with_buffer(pixel_buffer.make_mut_bytes(), size);
    let root = backend.into_drawing_area();
    root.fill(&WHITE).expect("error filling drawing area");

    let mut chart = ChartBuilder::on(&root)
        .build_cartesian_2d(0.0..30.0, -800.0..100.0)
        .expect("error building coordinate system");

    chart.configure_mesh().draw().expect("error drawing");

    for (channel, samples) in signal_data.iter().enumerate() {
        chart
            .draw_series(LineSeries::new(
                samples.iter().enumerate().map(|(i, x)| {
                    (
                        i as f64,
                        (*x as f64 - 127.0) / 1.27 - 100.0 * channel as f64,
                    )
                }),
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
    //slint::Image::load_from_svg_data(include_str!("data/icon.svg").as_bytes()).unwrap()
}
