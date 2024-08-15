#![doc(html_favicon_url = "https://psylink.me/favicon.ico")]
#![doc(html_logo_url = "https://psylink.me/favicon.ico")]

pub mod bluetooth;
pub mod firmware;
#[cfg(feature = "gui")]
pub mod gui;
#[allow(dead_code)]
pub mod protocol;

pub mod prelude {
    #[cfg(feature = "gui")]
    pub use crate::gui;
    pub use crate::{bluetooth, firmware, protocol};

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

    let conf = prelude::App {
        verbose: 0,
        scantime: 3.0,
    };
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            gui::start(conf).await;
        });
}
