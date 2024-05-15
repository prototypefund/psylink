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

pub fn scan(app: base::App) -> Result<(), Box<dyn Error>> {
    println!("Scanning...");

    let manager = app.rt.block_on(async { Manager::new().await })?;
    let adapter_list = app.rt.block_on(async {manager.adapters().await })?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    for adapter in adapter_list.iter() {
        println!("Trying bluetooth adapter {}...", app.rt.block_on(async { adapter.adapter_info().await })?);
        app.rt.block_on(async {
            adapter
                .start_scan(ScanFilter::default())
                .await
                .expect("Can't scan BLE adapter for connected devices...");
            time::sleep(Duration::from_secs_f32(app.scantime)).await;
        });

        let peripherals = app.rt.block_on(async { adapter.peripherals().await })?;
        if peripherals.is_empty() {
            eprintln!("No BLE peripheral devices found.");
        } else {
            for peripheral in peripherals.iter() {
                let properties = app.rt.block_on(async { peripheral.properties().await })?;
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

pub fn stream(app: base::App) -> Result<(), Box<dyn Error>> {
    println!("Scanning...");
    let manager = app.rt.block_on(async { Manager::new().await })?;
    let adapter_list = app.rt.block_on(async {manager.adapters().await })?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    let sensor_uuid = Uuid::parse_str(firmware::SENSOR_CHARACTERISTICS_UUID).unwrap();

    for adapter in adapter_list.iter() {
        println!("Trying bluetooth adapter {}...", app.rt.block_on(async { adapter.adapter_info().await })?);
        app.rt.block_on(async {
            adapter
                .start_scan(ScanFilter::default())
                .await
                .expect("Can't scan BLE adapter for connected devices...");
            time::sleep(Duration::from_secs_f32(app.scantime)).await;
        });

        let peripherals = app.rt.block_on(async { adapter.peripherals().await })?;
        if peripherals.is_empty() {
            eprintln!("No BLE peripheral devices found.");
            return Ok(());
        }
        let psylink = peripherals.iter().find(|peripheral| {
            let properties = app.rt.block_on(async {
                peripheral.properties().await
            });
            if let Ok(Some(PeripheralProperties { local_name: Some(name), .. })) = &properties {
                name == "PsyLink"
            } else {
                false
            }
        });

        if psylink.is_none() {
            continue;
        }
        let psylink = psylink.unwrap();
        let characteristics = app.rt.block_on(async {
            let _ = psylink.connect().await;
            let _ = psylink.discover_services().await;
            psylink.characteristics()
        });

        let sensor_characteristic = characteristics.iter().find(|c| c.uuid == sensor_uuid).unwrap();
        loop {
            let data = app.rt.block_on(async { psylink.read(sensor_characteristic).await }).unwrap();
            dbg!(data);
        }
    }
    Ok(())
}

pub fn find_peripheral(app: base::App) -> Result<btleplug::platform::Peripheral, Box<dyn Error>> {
    println!("Scanning...");

    let manager = app.rt.block_on(async { Manager::new().await })?;
    let adapter_list = app.rt.block_on(async {manager.adapters().await })?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    for adapter in adapter_list.iter() {
        println!("Trying bluetooth adapter {}...", app.rt.block_on(async { adapter.adapter_info().await })?);
        app.rt.block_on(async {
            adapter
                .start_scan(ScanFilter::default())
                .await
                .expect("Can't scan BLE adapter for connected devices...");
            time::sleep(Duration::from_secs_f32(app.scantime)).await;
        });

        let peripherals = app.rt.block_on(async { adapter.peripherals().await })?;
        if peripherals.is_empty() {
            eprintln!("No BLE peripheral devices found.");
        } else {
            for peripheral in peripherals.iter() {
                let properties = app.rt.block_on(async { peripheral.properties().await })?;
                if app.verbose > 2 {
                    dbg!(&properties);
                }
                if let Some(PeripheralProperties { address, local_name: Some(name), .. }) = &properties {
                    if name == "PsyLink" {
                        println!("Found PsyLink device with address {address}");
                        return Ok(peripheral.clone());
                    }
                }
            }
        }
    }
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "oh no!")));
}

//pub fn connect() {
//}
//
//pub fn get_current_scan_results() {
//}
