use crate::{base, bluetooth};
slint::include_modules!();

pub fn start(app: base::App) {
    let ui = MainWindow::new().unwrap();
    let ui_weak = ui.as_weak();

    app.rt.block_on(async {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                if let Ok(device) = bluetooth::find_peripheral().await {
                    ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_mytext(format!("Found PsyLink with MAC address {}.\n\nConnecting...", device.address).into());
                    }).unwrap();
                }
            }
        });
    });
    ui.run().unwrap();
}
