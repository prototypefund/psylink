use std::error::Error;
use std::time::Duration;
use btleplug::api::{Central, Manager as _, Peripheral, PeripheralProperties, ScanFilter};
use btleplug::platform::Manager;
use tokio::time;
use uuid::Uuid;
use crate::{base, firmware};

//pub struct State {
//    manager: Manager,
//}

#[derive(Clone)]
pub struct Device {
    pub name: String,
    pub address: String,
    peripheral: btleplug::platform::Peripheral,
}

pub async fn scan(app: base::App) -> Result<(), Box<dyn Error>> {
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
        time::sleep(Duration::from_secs_f32(app.scantime)).await;

        let peripherals = adapter.peripherals().await?;
        if peripherals.is_empty() {
            eprintln!("No BLE peripheral devices found.");
        } else {
            for peripheral in peripherals.iter() {
                let properties = peripheral.properties().await?;
                if app.verbose > 2 {
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

pub async fn stream(app: base::App) -> Result<(), Box<dyn Error>> {
    println!("Scanning...");
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    let sensor_uuid = Uuid::parse_str(firmware::SENSOR_CHARACTERISTICS_UUID).unwrap();

    let psylink = find_peripheral().await?;

    let _ = psylink.peripheral.connect().await;
    let _ = psylink.peripheral.discover_services().await;
    let characteristics = psylink.peripheral.characteristics();

    let sensor_characteristic = characteristics.iter().find(|c| c.uuid == sensor_uuid).unwrap();
    loop {
        let data = psylink.peripheral.read(sensor_characteristic).await?;
        dbg!(data);
    }
}

pub async fn find_peripheral() -> Result<Device, Box<dyn Error>> {
    println!("Scanning...");

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    loop {
        for adapter in adapter_list.iter() {
            println!("Trying bluetooth adapter {}...", adapter.adapter_info().await?);
            let _ = adapter.start_scan(ScanFilter::default()).await;
                //.expect("Can't scan BLE adapter for connected devices...");
            time::sleep(Duration::from_secs_f32(0.1)).await;

            let peripherals = adapter.peripherals().await?;
            if peripherals.is_empty() {
                eprintln!("No BLE peripheral devices found.");
            } else {
                for peripheral in peripherals.iter() {
                    let properties = peripheral.properties().await?;
                    dbg!(&properties);
                    if let Some(PeripheralProperties { address, local_name: Some(name), .. }) = &properties {
                        if name == "PsyLink" {
                            println!("Found PsyLink device with address {address}");
                            return Ok(Device {
                                name: name.to_string(),
                                address: address.to_string(),
                                peripheral: peripheral.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
}

//pub fn connect() {
//}
//
//pub fn get_current_scan_results() {
//}
