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
