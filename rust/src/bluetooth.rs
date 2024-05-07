use std::error::Error;
use std::time::Duration;
use btleplug::api::{Central, Manager as _, Peripheral, PeripheralProperties, ScanFilter};
use btleplug::platform::Manager;
use tokio::time;
use crate::base;

pub async fn scan(conf: base::Config) -> Result<(), Box<dyn Error>> {
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
        time::sleep(Duration::from_secs_f32(conf.scantime)).await;

        let peripherals = adapter.peripherals().await?;
        if peripherals.is_empty() {
            eprintln!("No BLE peripheral devices found.");
        } else {
            for peripheral in peripherals.iter() {
                let properties = peripheral.properties().await?;
                if conf.verbose > 2 {
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
    Ok(())
}
