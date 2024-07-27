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
