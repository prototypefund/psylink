slint::include_modules!();

pub fn start() {
    let app = MainWindow::new().unwrap();
    app.run().unwrap();
}
