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

    #[cfg(feature = "gui")]
    /// Open the graphical user interface (default action)
    Gui {
    },
}

mod base {
    pub struct Config {
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

    let conf = base::Config {
        verbose: cli.verbose,
        scantime: cli.scantime,
    };

    match &cli.command {
        Some(Commands::Scan { }) => {
            bluetooth::scan(conf).await?;
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
