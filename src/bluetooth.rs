use crate::prelude::*;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral, PeripheralProperties, ScanFilter,
};
use btleplug::platform::Manager;
use std::error::Error;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

//pub struct State {
//    manager: Manager,
//}

#[derive(Clone)]
pub struct Device {
    pub name: String,
    pub address: String,
    peripheral: btleplug::platform::Peripheral,
    characteristics: Option<Characteristics>,
}

#[derive(Clone)]
pub struct Characteristics {
    _channel_count: Characteristic,
    sensor: Characteristic,
}

impl Device {
    pub async fn find_characteristics(&mut self) {
        let uuid_sensor = Uuid::parse_str(firmware::SENSOR_CHARACTERISTICS_UUID).unwrap();
        let uuid_channel_count =
            Uuid::parse_str(firmware::CHANNEL_COUNT_CHARACTERISTICS_UUID).unwrap();

        let _ = self.peripheral.connect().await;
        let _ = self.peripheral.discover_services().await;
        let characteristics = self.peripheral.characteristics();
        let chr_sensor = characteristics
            .iter()
            .find(|c| c.uuid == uuid_sensor)
            .unwrap();
        let chr_channel_count = characteristics
            .iter()
            .find(|c| c.uuid == uuid_channel_count)
            .unwrap();

        self.characteristics = Some(Characteristics {
            _channel_count: chr_channel_count.clone(),
            sensor: chr_sensor.clone(),
        });
    }

    pub async fn read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        if let Some(chr) = &self.characteristics {
            Ok(self.peripheral.read(&chr.sensor).await?)
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Must load characteristics before calling read()",
            )))
        }
    }

    pub async fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        self.peripheral.disconnect().await?;
        Ok(())
    }
}

pub async fn scan(app: App) -> Result<(), Box<dyn Error>> {
    println!("Scanning Bluetooth for PsyLink device...");

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    for adapter in adapter_list.iter() {
        if app.verbose > 0 {
            println!(
                "Trying bluetooth adapter {}...",
                adapter.adapter_info().await?
            );
        }
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
                if app.verbose > 1 {
                    dbg!(&properties);
                }
                if let Some(PeripheralProperties {
                    address,
                    local_name: Some(name),
                    ..
                }) = &properties
                {
                    if name == "PsyLink" {
                        println!("Found PsyLink device with address {address}");
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn stream(app: App) -> Result<(), Box<dyn Error>> {
    println!("Scanning Bluetooth for PsyLink device...");
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    let sensor_uuid = Uuid::parse_str(firmware::SENSOR_CHARACTERISTICS_UUID).unwrap();

    let psylink = find_peripheral(app, None).await?;

    let _ = psylink.peripheral.connect().await;
    let _ = psylink.peripheral.discover_services().await;
    let characteristics = psylink.peripheral.characteristics();

    let sensor_characteristic = characteristics
        .iter()
        .find(|c| c.uuid == sensor_uuid)
        .unwrap();
    loop {
        let data = psylink.peripheral.read(sensor_characteristic).await?;
        dbg!(data);
    }
}

pub async fn find_peripheral(
    app: App,
    mutex_quit: Option<Arc<Mutex<bool>>>,
) -> Result<Device, Box<dyn Error>> {
    println!("Scanning Bluetooth for PsyLink device...");

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    loop {
        for adapter in adapter_list.iter() {
            if app.verbose > 0 {
                println!(
                    "Trying Bluetooth adapter {}...",
                    adapter.adapter_info().await?
                );
            }
            let _ = adapter.start_scan(ScanFilter::default()).await;
            //.expect("Can't scan BLE adapter for connected devices...");
            time::sleep(Duration::from_secs_f32(0.1)).await;

            let peripherals = adapter.peripherals().await?;
            if peripherals.is_empty() {
                eprintln!("No BLE peripheral devices found.");
            } else {
                for peripheral in peripherals.iter() {
                    let properties = peripheral.properties().await?;
                    if app.verbose > 1 {
                        dbg!(&properties);
                    }
                    if let Some(PeripheralProperties {
                        address,
                        local_name: Some(name),
                        ..
                    }) = &properties
                    {
                        if name == "PsyLink" {
                            println!("Found PsyLink device with address {address}");
                            return Ok(Device {
                                name: name.to_string(),
                                address: address.to_string(),
                                peripheral: peripheral.clone(),
                                characteristics: None,
                            });
                        }
                    }
                }
            }
        }
        if let Some(mutex_quit) = &mutex_quit {
            if *(mutex_quit.lock().unwrap()) {
                println!("Quitting bluetooth::Device::find_peripheral");
                return Err(Box::new(io::Error::new(io::ErrorKind::Other, "The bluetooth::Device::find_peripheral function received a quit command while scanning for peripherals")));
            }
        }
    }
}
