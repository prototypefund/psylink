use crate::base;
slint::include_modules!();

pub fn start(app: base::App) {
    let ui = MainWindow::new().unwrap();
    let ui_weak = ui.as_weak();

    app.rt.block_on(async {
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_mytext("Found PsyLink with MAC address 11:22:33:44:55:66.\n\nConnecting...".into());
            }).unwrap();
        });

        ui.run().unwrap();
    });
}
