mod firmware;
mod bluetooth;
#[cfg(feature = "gui")]
mod gui;

use std::error::Error;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Display more information on the console. Can be used multiple times.
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Seconds to wait when scanning for bluetooth devices
    #[arg(short, long, value_name = "SECONDS", default_value_t = 3.0)]
    scantime: f32,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan for PsyLink devices
    Scan {
    },

    /// Write the raw data from a PsyLink to the console
    Print {
    },

    #[cfg(feature = "gui")]
    /// Open the graphical user interface (default action)
    Gui {
    },
}

mod base {
    pub struct App {
        pub verbose: u8,
        pub scantime: f32,
        pub rt: tokio::runtime::Runtime,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    if cli.verbose > 2 {
        dbg!(&cli);
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let conf = base::App {
        verbose: cli.verbose,
        scantime: cli.scantime,
        rt: rt,
    };

    match &cli.command {
        Some(Commands::Scan { }) => {
            let _ = bluetooth::scan(conf);
        }
        Some(Commands::Print { }) => {
            let _ = bluetooth::stream(conf);
        }
        #[cfg(feature = "gui")]
        Some(Commands::Gui { }) | None => {
            gui::start();
        }
        #[cfg(not(feature = "gui"))]
        None => {
            <Cli as clap::CommandFactory>::command().print_help()?;
        }
    }

    Ok(())
}
