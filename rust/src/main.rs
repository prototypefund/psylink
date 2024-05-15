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
    #[derive(Clone, Copy)]
    pub struct App {
        pub verbose: u8,
        pub scantime: f32,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    if cli.verbose > 2 {
        dbg!(&cli);
    }

    let conf = base::App {
        verbose: cli.verbose,
        scantime: cli.scantime,
    };

    match &cli.command {
        Some(Commands::Scan { }) => {
            bluetooth::scan(conf).await?;
        }
        Some(Commands::Print { }) => {
            bluetooth::stream(conf).await?;
        }
        #[cfg(feature = "gui")]
        Some(Commands::Gui { }) | None => {
            gui::start(conf).await;
        }
        #[cfg(not(feature = "gui"))]
        None => {
            <Cli as clap::CommandFactory>::command().print_help()?;
        }
    }

    Ok(())
}
