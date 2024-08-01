#![doc(html_favicon_url = "https://psylink.me/favicon.ico")]
#![doc(html_logo_url = "https://psylink.me/favicon.ico")]

pub mod bluetooth;
pub mod firmware;
#[cfg(feature = "gui")]
pub mod gui;
#[allow(dead_code)]
pub mod protocol;

pub mod prelude {
    pub use crate::{bluetooth, firmware, protocol};
    #[cfg(feature = "gui")]
    pub use crate::gui;

    #[derive(Clone, Copy)]
    pub struct App {
        pub verbose: u8,
        pub scantime: f32,
    }
}

#[no_mangle]
#[cfg(target_os = "android")]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();

    // ... rest of your code ...
    slint::slint!{
        export component MainWindow inherits Window {
            Text { text: "Hello World"; }
        }
    }
    MainWindow::new().unwrap().run().unwrap();
}
