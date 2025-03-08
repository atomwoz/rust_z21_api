use std::{
    io::{stdout, Write},
    sync::Arc,
    time::Duration,
};

use tokio::{io::AsyncWriteExt, time};
use z21_api::{Loco, Z21Station};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the station by binding to the specified address.
    let station = Z21Station::new("192.168.0.111:21105").await?;
    let station = Arc::new(station);

    // Retrieve and print the serial number from the station.
    match station.get_serial_number().await {
        Ok(sn) => println!("Serial number: {}", sn),
        Err(e) => eprintln!("Error: {:?}", e),
    }

    let rag_loco = Loco::control(Arc::clone(&station), 4).await?;

    station.subscribe_system_state(
        5.,
        Box::new(|state| {
            println!("System state: {:?}", state);
        }),
    );
    // rag_loco.subscribe_loco_state(Box::new(|state| {
    //     print!(
    //         "\rLoco state: Speed:{}%  Busy:{}   Lights:{}    F5:{}      ",
    //         ((state.speed_percentage.unwrap_or(0.) * 100.).round() / 100.),
    //         state.is_busy.unwrap_or(false),
    //         if state.functions.unwrap_or([false; 32])[0] {
    //             "💡"
    //         } else {
    //             "🌚"
    //         },
    //         state.functions.unwrap_or([false; 32])[5]
    //     );
    //     stdout().flush().unwrap();
    // }));

    loop {
        // rag_loco.drive(25.).await?;
        // time::sleep(Duration::from_millis(1500)).await;
        // rag_loco.halt().await?;
        // time::sleep(Duration::from_millis(1500)).await;
        // rag_loco.drive(-25.).await?;
        // time::sleep(Duration::from_millis(1500)).await;
        // rag_loco.halt().await?;
        // time::sleep(Duration::from_millis(1500)).await;
    }

    //Ok(())
}
