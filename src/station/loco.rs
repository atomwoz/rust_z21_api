use std::time::Duration;
use std::{ops::Deref, sync::Arc, vec};

use tokio::{io, time};

use crate::messages::{DccThrottleSteps, LocoState};
use crate::{messages::XBusMessage, Z21Station};

const XBUS_LOCO_GET_INFO: u8 = 0xE3;
const XBUS_LOCO_DRIVE: u8 = 0xE4;
const XBUS_LOCO_INFO: u8 = 0xEF;

impl Default for DccThrottleSteps {
    fn default() -> Self {
        Self::Steps128
    }
}
/// Represents DCC Locomotive.
pub struct Loco {
    station: Arc<Z21Station>,
    addr: u16,
    steps: DccThrottleSteps,
}

impl Loco {
    /// Initializing control over loco with specified address.  
    /// It also subscribes info about it.
    pub async fn control(station: Arc<Z21Station>, address: u16) -> io::Result<Loco> {
        Self::control_with_steps(station, address, DccThrottleSteps::default()).await
    }

    /// Initializing control over loco with specified address and DCC stepping.  
    /// It also subscribes info about it.
    pub async fn control_with_steps(
        station: Arc<Z21Station>,
        address: u16,
        steps: DccThrottleSteps,
    ) -> io::Result<Loco> {
        let loco = Loco {
            station: station.clone(),
            steps,
            addr: address,
        };

        Self::poll_state_info(address, &loco.station).await?;
        Ok(loco)
    }

    async fn send_drive(&self, drive_byte: u8) -> io::Result<()> {
        let addr_bytes = self.addr.to_be_bytes();
        let dbs = vec![self.steps as u8, addr_bytes[0], addr_bytes[1], drive_byte];
        let drive_msg = XBusMessage::new_dbs_vec(XBUS_LOCO_DRIVE, dbs);
        self.station
            .send_xbus_command(drive_msg, Some(XBUS_LOCO_INFO))
            .await?;
        Ok(())
    }
    /// Normal loco stop, equivalent to setting speed to 0.  
    /// It applies braking with a braking curve.  
    ///
    /// # Errors
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.  
    pub async fn stop(&self) -> io::Result<()> {
        self.send_drive(0x0).await
    }
    /// Stops the train immediately (emergency stop).
    ///
    /// # Errors
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.  
    pub async fn halt(&self) -> io::Result<()> {
        self.send_drive(0x1).await
    }
    fn calc_speed(steps: DccThrottleSteps, speed_percent: f64) -> u8 {
        let speed = speed_percent / 100.;
        let mapped_speed = match steps {
            DccThrottleSteps::Steps128 => speed * 128.,
            DccThrottleSteps::Steps28 => speed * 28.,
            DccThrottleSteps::Steps14 => speed * 14.,
        };
        //let mapped_speed = (mapped_speed * 100.).round() / 100.;
        let flag = mapped_speed > 0.;

        let to_out = (mapped_speed.abs() as u8) | (0x80 * flag as u8);
        to_out
    }

    async fn poll_state_info(addr: u16, station: &Arc<Z21Station>) -> io::Result<LocoState> {
        let addr_bytes = addr.to_be_bytes();
        let init_xbus =
            XBusMessage::new_dbs_vec(XBUS_LOCO_GET_INFO, vec![0xf0, addr_bytes[0], addr_bytes[1]]);
        let info = station
            .send_xbus_command(init_xbus, Some(XBUS_LOCO_INFO))
            .await?;

        Ok(LocoState::try_from(&info)?)
    }

    /// Sets speed of the locomotive in percent.  
    /// It is automatically calculated based on the number of steps.  
    /// When the speed is positive, the locomotive moves forward.  
    /// When the speed is negative, the locomotive moves backward.  
    /// When setting the speed to 0, the locomotive stops using a braking curve.  
    /// To stop the locomotive immediately, use the `halt` method.  
    ///
    /// # Errors
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    /// For example, to drive forward at 50% speed:
    /// ```rust
    /// loco.drive(50.0).await?;
    /// ```
    pub async fn drive(&self, speed_percent: f64) -> io::Result<()> {
        let calced = Self::calc_speed(self.steps, speed_percent);
        self.send_drive(calced).await?;
        Ok(())
    }

    pub fn subscribe_loco_state(&self, subscriber: Box<dyn Fn(LocoState) + Send + Sync>) {
        let station = Arc::clone(&self.station);
        tokio::spawn(async move {
            loop {
                let msg = station.receive_xbus_packet(XBUS_LOCO_INFO).await;
                if let Ok(msg) = msg {
                    if let Ok(loco_state) = LocoState::try_from(&msg) {
                        subscriber(loco_state);
                    }
                }
            }
        });
    }
}
