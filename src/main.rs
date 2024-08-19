use psylink::prelude::*;

use clap::{Parser, Subcommand};
use std::error::Error;

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
    Scan {},

    /// Write the raw data from a PsyLink to the console
    Print {},

    /// Perform a calibration on the test dataset
    Train {},

    /// Perform a calibration inference based on the pre-trained test model
    Infer {},

    #[cfg(feature = "gui")]
    /// Open the graphical user interface (default action)
    Gui {},
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    if cli.verbose > 1 {
        dbg!(&cli);
    }

    let conf = App {
        verbose: cli.verbose,
        scantime: cli.scantime,
    };

    match &cli.command {
        Some(Commands::Scan {}) => {
            bluetooth::scan(conf).await?;
        }
        Some(Commands::Print {}) => {
            bluetooth::stream(conf).await?;
        }
        Some(Commands::Train {}) => {
            calibration::train()?;
        }
        Some(Commands::Infer {}) => {
            calibration::infer()?;
        }
        #[cfg(feature = "gui")]
        Some(Commands::Gui {}) | None => {
            gui::start(conf).await;
        }
        #[cfg(not(feature = "gui"))]
        None => {
            <Cli as clap::CommandFactory>::command().print_help()?;
        }
    }

    Ok(())
}
