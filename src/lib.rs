#![doc(html_favicon_url = "https://psylink.me/favicon.ico")]
#![doc(html_logo_url = "https://psylink.me/favicon.ico")]

pub mod bluetooth;
pub mod calibration;
pub mod fakeinput;
pub mod firmware;
#[cfg(feature = "gui")]
pub mod gui;
#[allow(dead_code)]
pub mod protocol;
pub mod sound;

pub mod prelude {
    pub use crate::fakeinput::Action;
    #[cfg(feature = "gui")]
    pub use crate::gui;
    pub use crate::{bluetooth, calibration, fakeinput, firmware, protocol, sound};

    #[derive(Clone, Copy)]
    pub struct App {
        pub verbose: u8,
        pub scantime: f32,
    }

    pub fn transpose_vec<T: Clone>(matrix: Vec<Vec<T>>) -> Vec<Vec<T>> {
        if matrix.is_empty() || matrix[0].is_empty() {
            return vec![];
        }

        let col_count = matrix[0].len();

        // Create a new matrix with dimensions swapped
        let mut transposed = vec![vec![]; col_count];

        for row in matrix {
            for (j, item) in row.into_iter().enumerate() {
                transposed[j].push(item);
            }
        }

        transposed
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
