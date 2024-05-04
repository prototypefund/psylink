use std::error::Error;
use std::time::Duration;
use btleplug::api::{Central, Manager as _, Peripheral, PeripheralProperties, ScanFilter};
use btleplug::platform::Manager;
use clap::{Parser, Subcommand};
use tokio::time;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
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
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    if cli.verbose > 2 {
        dbg!(&cli);
    }

    match &cli.command {
        Some(Commands::Scan { }) => {
            println!("Scanning...");
            let manager = Manager::new().await?;
            let adapter_list = manager.adapters().await?;
            if adapter_list.is_empty() {
                eprintln!("No Bluetooth adapters found");
            }

            for adapter in adapter_list.iter() {
                println!("Trying bluetooth adapter {}...", adapter.adapter_info().await?);
                adapter
                    .start_scan(ScanFilter::default())
                    .await
                    .expect("Can't scan BLE adapter for connected devices...");
                time::sleep(Duration::from_secs_f32(cli.scantime)).await;

                let peripherals = adapter.peripherals().await?;
                if peripherals.is_empty() {
                    eprintln!("No BLE peripheral devices found.");
                } else {
                    for peripheral in peripherals.iter() {
                        let properties = peripheral.properties().await?;
                        if cli.verbose > 2 {
                            dbg!(&properties);
                        }
                        if let Some(PeripheralProperties { address, local_name: Some(name), .. }) = &properties {
                            if name == "PsyLink" {
                                println!("Found PsyLink device with address {address}");
                            }
                        }
                    }
                }
            }
        }
        None => {}
    }

    Ok(())
}
