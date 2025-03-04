use std::time::Duration;

use tokio::time;
use z21_api::Z21Station;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the station by binding to the specified address.
    let station = Z21Station::new("192.168.0.111:21105").await?;

    // Retrieve and print the serial number from the station.
    match station.get_serial_number().await {
        Ok(sn) => println!("Serial number: {}", sn),
        Err(e) => eprintln!("Error: {:?}", e),
    }

    loop {
        station.voltage_off().await?;
        time::sleep(Duration::from_millis(1500)).await;
        station.voltage_on().await?;
        time::sleep(Duration::from_millis(1500)).await;
    }

    //Ok(())
}
