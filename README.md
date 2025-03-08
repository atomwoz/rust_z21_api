# Z21Station

[![Crates.io](https://img.shields.io/crates/v/z21_driver.svg)](https://crates.io/crates/roco_z21_driver)
[![Documentation](https://docs.rs/roco_z21_driver/badge.svg)](https://docs.rs/roco_z21_driver)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/roco_z21_driver.svg)](./LICENSE)

A Rust library for asynchronous communication with a Roco Fleischmann Z21 digital command control (DCC) station for model railways.

## Overview

This crate provides a complete UDP-based API for interacting with the Z21 station, handling command transmission and event reception through an asynchronous architecture powered by the Tokio runtime.

## Features

- Automatic connection management with keep-alive functionality
- Broadcast message handling for system state changes
- Locomotive control (speed, direction, functions)
- Support for different DCC throttle steps (14, 28, 128)
- Track power control
- Asynchronous, subscription-based event handling
- Error handling
- Ready to use driver for integration into other projects

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
z21_driver = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

## Usage Examples

### Basic Connection

```rust
use roco_z21_driver::Z21Station;
use std::sync::Arc;
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create a connection to the Z21 station
    let station = Arc::new(Z21Station::new("192.168.0.111:21105").await?);

    // Get the serial number of the station
    let serial = station.get_serial_number().await?;
    println!("Z21 station serial number: {}", serial);

    // Turn on track power
    station.voltage_on().await?;

    // Subscribe to system state updates
    station.subscribe_system_state(
        1.0,
        Box::new(|state| {
            println!("Main track voltage: {:.2}V", state.vcc_voltage);
            println!("Temperature: {}°C", state.temperature);
            println!("Current: {}mA", state.main_current);
        }),
    );

    // Keep the application running
    tokio::signal::ctrl_c().await?;

    // Turn off track power before exiting
    station.voltage_off().await?;
    station.logout().await?;

    Ok(())

    //Ok(())
}

```

### Controlling a Locomotive

```rust
use roco_z21_driver::{Loco, Z21Station};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let station = Arc::new(Z21Station::new("192.168.0.111:21105").await?);

    // Control a locomotive with address 3
    let loco = Loco::control(station.clone(), 4).await?;

    // Subscribe to locomotive state changes
    loco.subscribe_loco_state(Box::new(|state| {
        println!(
            "Locomotive speed: {}%",
            state.speed_percentage.unwrap_or(0.)
        );
    }));

    // Turn on the headlights
    loco.set_headlights(true).await?;

    // Set speed to 50% forward
    loco.drive(50.0).await?;

    // Wait for 5 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Gradually stop
    loco.stop().await?;

    Ok(())
}


```

## API Documentation

### Z21Station

The `Z21Station` struct provides methods to interact with the Z21 station:

- `new(bind_addr: &str) -> io::Result<Self>`: Creates a new connection to a Z21 station
- `voltage_off() -> io::Result<()>`: Turns off the track voltage (emergency stop)
- `voltage_on() -> io::Result<()>`: Turns on the track voltage
- `get_serial_number() -> io::Result<u32>`: Retrieves the serial number from the Z21 station
- `subscribe_system_state(freq_in_sec: f64, subscriber: Box<dyn Fn(SystemState) + Send + Sync>)`: Subscribes to system state updates
- `logout() -> io::Result<()>`: Logs out from the Z21 station

### Locomotive Control

The `Loco` struct provides methods to control DCC locomotives:

- `control(station: Arc<Z21Station>, address: u16) -> io::Result<Loco>`: Controls a locomotive with default throttle steps (128)
- `control_with_steps(station: Arc<Z21Station>, address: u16, steps: DccThrottleSteps) -> io::Result<Loco>`: Controls with specific throttle steps
- `drive(speed_percent: f64) -> io::Result<()>`: Sets the speed of the locomotive (-100.0 to 100.0)
- `stop() -> io::Result<()>`: Performs a normal locomotive stop
- `halt() -> io::Result<()>`: Stops the train immediately (emergency stop)
- `set_function(function_index: u8, action: u8) -> io::Result<()>`: Controls a locomotive function (F0-F31)
- `function_on(function_index: u8) -> io::Result<()>`: Turns on a specific locomotive function
- `function_off(function_index: u8) -> io::Result<()>`: Turns off a specific locomotive function
- `function_toggle(function_index: u8) -> io::Result<()>`: Toggles a specific locomotive function
- `set_headlights(on: bool) -> io::Result<()>`: Convenience method to control the locomotive's headlights (F0)
- `subscribe_loco_state(subscriber: Box<dyn Fn(LocoState) + Send + Sync>)`: Subscribes to locomotive state changes

## License

This project is licensed under either of:

-BSD 3-Clause License, see [LICENSE-BSD](https://opensource.org/license/bsd-3-clause) file

at your option.

## Contributions

Contributions are welcome! Please feel free to submit a Pull Request.
