use crate::{base, bluetooth};
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
                    ui.set_mytext(
                        format!("Found PsyLink with MAC address {address}.\n\nConnecting...")
                            .into(),
                    );
                });
                break device;
            }
        };
        device.find_characteristics().await;

        loop {
            let bytearray: Vec<u8> = device.read().await.unwrap();
            let mut string = String::new();
            for (i, byte) in bytearray.iter().enumerate() {
                string += byte.to_string().as_str();
                if i % 20 == 19 {
                    string += ",\n";
                } else {
                    string += ", ";
                }
            }
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_mytext(string.into());
            });
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.1)).await;
        }
    });
    ui.run().unwrap();
}
