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

    station
        .subscribe_to_system_state(|x| {
            println!("System state: {:?}", x);
        })
        .expect("Failed to subscribe to system state");

    let status = station.get_system_status().await?;
    println!("System status: {:?}", status);
    tokio::time::sleep(std::time::Duration::from_secs(100000000000)).await;
    Ok(())
}
