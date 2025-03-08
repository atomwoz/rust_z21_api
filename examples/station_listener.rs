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
