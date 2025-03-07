use std::{ops::Deref, sync::Arc, vec};

use tokio::io;

use crate::{messages::XBusMessage, Z21Station};

const XBUS_LOCO_GET_INFO: u8 = 0xE3;
const XBUS_LOCO_DRIVE: u8 = 0xE4;
const XBUS_LOCO_INFO: u8 = 0xEF;

#[derive(Clone, Copy)]
pub enum DccThrottleSteps {
    Steps14 = 0x10,
    Steps28 = 0x12,
    Steps128 = 0x13,
}

impl Default for DccThrottleSteps {
    fn default() -> Self {
        Self::Steps128
    }
}
/// Represents DCC Locomotive.
pub struct Loco<'a> {
    station: &'a Z21Station,
    addr: u16,
    steps: DccThrottleSteps,
}

impl<'a> Loco<'a> {
    /// Initializing control over loco with specified address.  
    /// It also subscribes info about it.
    pub async fn control(station: &Z21Station, address: u16) -> io::Result<Loco> {
        Self::control_with_steps(station, address, DccThrottleSteps::default()).await
    }
    /// Initializing control over loco with specified address and DCC stepping.  
    /// It also subscribes info about it.
    pub async fn control_with_steps(
        station: &Z21Station,
        address: u16,
        steps: DccThrottleSteps,
    ) -> io::Result<Loco> {
        let loco = Loco {
            station,
            steps,
            addr: address,
        };
        let addr_bytes = address.to_be_bytes();
        let init_xbus =
            XBusMessage::new_dbs_vec(XBUS_LOCO_GET_INFO, vec![0xf0, addr_bytes[0], addr_bytes[1]]);
        station
            .send_xbus_command(init_xbus, Some(XBUS_LOCO_INFO))
            .await?;
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
}
