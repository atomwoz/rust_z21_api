use std::time::Duration;

use tokio::time;
use z21_api::{Loco, Z21Station};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the station by binding to the specified address.
    let station = Z21Station::new("192.168.0.111:21105").await?;

    // Retrieve and print the serial number from the station.
    match station.get_serial_number().await {
        Ok(sn) => println!("Serial number: {}", sn),
        Err(e) => eprintln!("Error: {:?}", e),
    }

    let rag_loco = Loco::control(&station, 4).await?;

    loop {
        rag_loco.drive(25.).await?;
        time::sleep(Duration::from_millis(1500)).await;
        rag_loco.halt().await?;
        time::sleep(Duration::from_millis(1500)).await;
        rag_loco.drive(-25.).await?;
        time::sleep(Duration::from_millis(1500)).await;
        rag_loco.halt().await?;
        time::sleep(Duration::from_millis(1500)).await;
    }

    //Ok(())
}
