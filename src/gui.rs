use crate::{base, bluetooth, protocol};
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
            ui.global::<Logic>().on_render_signal(|_channel_index| -> slint::Image {
                slint::Image::load_from_svg_data(include_str!("data/icon.svg").as_bytes()).unwrap()
            });
        });
        let mut decoder = protocol::Decoder::new(8);

        loop {
            let bytearray: Vec<u8> = device.read().await.unwrap(); // TODO: catch panic
            let text = bytearray.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(", ");
            println!("Received BLE payload: {};", text);

            let packet = decoder.decode_packet(bytearray);
            if packet.is_err() {
                let message: String = packet.unwrap_err();
                println!("Error: {message}");
                continue;
            }
            let packet = packet.unwrap();

            let mut string = String::new();
            for data in packet.samples {
                string += format!("{data:?}\n").as_str();
            }
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_datatext(string.into());
            });
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.1)).await;
        }
    });
    ui.run().unwrap();
}
