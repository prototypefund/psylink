use crate::{base, bluetooth};
slint::include_modules!();

pub async fn start(app: base::App) {
    let ui = MainWindow::new().unwrap();
    let ui_weak = ui.as_weak();

    let appclone = app.clone();
    tokio::spawn(async move {
        let device = loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            if let Ok(device) = bluetooth::find_peripheral(appclone).await {
                let address = device.address.clone();
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_mytext(format!("Found PsyLink with MAC address {address}.\n\nConnecting...").into());
                });
                break device;
            }
        };

    });
    ui.run().unwrap();
}
