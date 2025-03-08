//! Module for controlling DCC locomotives via the Z21 station.
//!
//! This module provides a high-level API for controlling model train locomotives
//! using the Digital Command Control (DCC) protocol via a Z21 station. It supports
//! operations such as controlling locomotive speed, direction, functions (lights,
//! sounds, etc.), and emergency stops.
//!
//! # Features
//!
//! - Control locomotive speed and direction
//! - Normal and emergency stops
//! - Function control (F0-F31) including lights, sounds, and other locomotive features
//! - Support for different DCC throttle steps (14, 28, 128)
//! - State monitoring and subscription
//!
//! # Examples
//!
//! ```rust
//! # use tokio;
//! # use std::sync::Arc;
//! # async fn example() -> std::io::Result<()> {
//! let station = Arc::new(Z21Station::new("192.168.0.111:21105").await?);
//!
//! // Control a locomotive with address 3
//! let loco = Loco::control(station.clone(), 3).await?;
//!
//! // Set speed to 50% forward
//! loco.drive(50.0).await?;
//!
//! // Turn on the headlights (F0)
//! loco.set_headlights(true).await?;
//!
//! // Activate the horn (assuming it's on F2)
//! loco.function_on(2).await?;
//!
//! // Emergency stop
//! loco.halt().await?;
//! # Ok(())
//! # }
//! ```

use std::time::Duration;
use std::{ops::Deref, sync::Arc, vec};

use tokio::{io, time};

use crate::messages::{DccThrottleSteps, LocoState};
use crate::{messages::XBusMessage, Z21Station};

const XBUS_LOCO_GET_INFO: u8 = 0xE3;
const XBUS_LOCO_DRIVE: u8 = 0xE4;
const XBUS_LOCO_INFO: u8 = 0xEF;
const XBUS_LOCO_FUNCTION: u8 = 0xE4;
const FUNC_OFF: u8 = 0x00;
const FUNC_ON: u8 = 0x01;
const FUNC_TOGGLE: u8 = 0x02;

impl Default for DccThrottleSteps {
    fn default() -> Self {
        Self::Steps128
    }
}

/// Represents a DCC Locomotive that can be controlled via a Z21 station.
///
/// This struct provides methods to control various aspects of a model train locomotive,
/// including speed, direction, functions (lights, sounds, etc.), and emergency stops.
/// It communicates with the locomotive through a Z21 station using the XBus protocol.
pub struct Loco {
    /// Reference to the Z21 station connection
    station: Arc<Z21Station>,
    /// DCC address of the locomotive
    addr: u16,
    /// DCC throttle steps configuration (14, 28, or 128 steps)
    steps: DccThrottleSteps,
}

impl Loco {
    /// Initializes control over a locomotive with the specified address.
    ///
    /// This method establishes communication with a locomotive using its DCC address
    /// and subscribes to information about its state. It uses the default throttle
    /// steps configuration (128 steps).
    ///
    /// # Arguments
    ///
    /// * `station` - Arc reference to a connected Z21Station
    /// * `address` - DCC address of the locomotive (1-9999)
    ///
    /// # Returns
    ///
    /// A new `Loco` instance if successful.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if:
    /// - Communication with the Z21 station fails
    /// - The locomotive does not respond
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(station: Arc<Z21Station>) -> std::io::Result<()> {
    /// let loco = Loco::control(station.clone(), 3).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn control(station: Arc<Z21Station>, address: u16) -> io::Result<Loco> {
        Self::control_with_steps(station, address, DccThrottleSteps::default()).await
    }

    /// Initializes control over a locomotive with specified address and DCC stepping.
    ///
    /// Similar to `control()` but allows specifying the throttle stepping mode
    /// (14, 28, or 128 steps) for more precise control or compatibility with
    /// different locomotive decoders.
    ///
    /// # Arguments
    ///
    /// * `station` - Arc reference to a connected Z21Station
    /// * `address` - DCC address of the locomotive (1-9999)
    /// * `steps` - DCC throttle steps configuration
    ///
    /// # Returns
    ///
    /// A new `Loco` instance if successful.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if:
    /// - Communication with the Z21 station fails
    /// - The locomotive does not respond
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(station: Arc<Z21Station>) -> std::io::Result<()> {
    /// let loco = Loco::control_with_steps(
    ///     station.clone(),
    ///     3,
    ///     DccThrottleSteps::Steps28
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Sends a drive command to the locomotive.
    ///
    /// Internal helper method used by `drive()`, `stop()`, and `halt()` methods.
    ///
    /// # Arguments
    ///
    /// * `drive_byte` - Control byte for the drive command
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    async fn send_drive(&self, drive_byte: u8) -> io::Result<()> {
        let addr_bytes = self.addr.to_be_bytes();
        let dbs = vec![self.steps as u8, addr_bytes[0], addr_bytes[1], drive_byte];
        let drive_msg = XBusMessage::new_dbs_vec(XBUS_LOCO_DRIVE, dbs);
        self.station
            .send_xbus_command(drive_msg, Some(XBUS_LOCO_INFO))
            .await?;
        Ok(())
    }

    /// Performs a normal locomotive stop, equivalent to setting speed to 0.
    ///
    /// This stop applies braking with a braking curve, providing a gradual
    /// and realistic deceleration.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Gradually stop the locomotive
    /// loco.stop().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stop(&self) -> io::Result<()> {
        self.send_drive(0x0).await
    }

    /// Stops the train immediately (emergency stop).
    ///
    /// Unlike the normal `stop()` method, this immediately cuts power
    /// to the locomotive, causing an abrupt stop. This should be used
    /// only in emergency situations.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Emergency stop the locomotive
    /// loco.halt().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn halt(&self) -> io::Result<()> {
        self.send_drive(0x1).await
    }

    /// Calculates the speed byte for a locomotive based on throttle steps and speed percentage.
    ///
    /// This function maps a percentage speed value (-100% to 100%) to the appropriate
    /// DCC speed step value based on the configured throttle steps. Negative values
    /// indicate reverse direction, positive values indicate forward direction.
    ///
    /// # Arguments
    ///
    /// * `steps` - DCC throttle steps configuration (14, 28, or 128 steps)
    /// * `speed_percent` - Speed percentage (-100.0 to 100.0)
    ///
    /// # Returns
    ///
    /// A formatted drive byte for the DCC command
    fn calc_speed(steps: DccThrottleSteps, speed_percent: f64) -> u8 {
        let speed = speed_percent / 100.;
        let mapped_speed = match steps {
            DccThrottleSteps::Steps128 => speed * 128.,
            DccThrottleSteps::Steps28 => speed * 28.,
            DccThrottleSteps::Steps14 => speed * 14.,
        };
        //let mapped_speed = (mapped_speed * 100.).round() / 100.;
        let flag = mapped_speed > 0.;

        (mapped_speed.abs() as u8) | (0x80 * flag as u8)
    }

    /// Polls the current state information of a locomotive.
    ///
    /// This method sends a request to the Z21 station to get the current state
    /// of a locomotive with the specified address.
    ///
    /// # Arguments
    ///
    /// * `addr` - DCC address of the locomotive
    /// * `station` - Reference to the Z21 station
    ///
    /// # Returns
    ///
    /// The current state of the locomotive if successful.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the request fails or the response is invalid.
    async fn poll_state_info(addr: u16, station: &Arc<Z21Station>) -> io::Result<LocoState> {
        let addr_bytes = addr.to_be_bytes();
        let init_xbus =
            XBusMessage::new_dbs_vec(XBUS_LOCO_GET_INFO, vec![0xf0, addr_bytes[0], addr_bytes[1]]);
        let info = station
            .send_xbus_command(init_xbus, Some(XBUS_LOCO_INFO))
            .await?;

        Ok(LocoState::try_from(&info)?)
    }

    /// Sets the speed of the locomotive in percent.
    ///
    /// This method controls both the speed and direction of the locomotive:
    /// - Positive values move the locomotive forward
    /// - Negative values move the locomotive backward
    /// - Zero value gradually stops the locomotive using a braking curve
    ///
    /// The speed is automatically scaled based on the configured DCC throttle steps.
    ///
    /// # Arguments
    ///
    /// * `speed_percent` - Speed percentage (-100.0 to 100.0)
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Drive forward at 50% speed
    /// loco.drive(50.0).await?;
    ///
    /// // Drive backward at 25% speed
    /// loco.drive(-25.0).await?;
    ///
    /// // Stop gradually
    /// loco.drive(0.0).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn drive(&self, speed_percent: f64) -> io::Result<()> {
        let calced = Self::calc_speed(self.steps, speed_percent);
        self.send_drive(calced).await?;
        Ok(())
    }

    /// Subscribes to locomotive state changes.
    ///
    /// This method sets up a background task that listens for locomotive state
    /// events from the Z21 station and calls the provided callback function
    /// whenever the state changes.
    ///
    /// # Arguments
    ///
    /// * `subscriber` - Callback function that receives locomotive state updates
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn example(loco: &Loco) {
    /// loco.subscribe_loco_state(Box::new(|state| {
    ///     println!("Locomotive speed: {}, direction: {}",
    ///              state.speed,
    ///              if state.direction { "forward" } else { "backward" });
    /// }));
    /// # }
    /// ```
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

    /// Controls a locomotive function (F0-F31).
    ///
    /// This method allows controlling the various functions of a DCC locomotive,
    /// such as lights, sounds, couplers, smoke generators, and other features.
    /// The specific functions available depend on the locomotive decoder.
    ///
    /// # Arguments
    ///
    /// * `function_index` - The function number (0-31) where 0 represents F0 (typically lights)
    /// * `action` - The action to perform:
    ///   - 0: Turn function OFF
    ///   - 1: Turn function ON
    ///   - 2: Toggle function state
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if:
    /// - The function index is invalid (must be 0-31)
    /// - The action is invalid (must be 0-2)
    /// - The packet fails to send
    /// - The Z21 station does not respond
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Turn on the locomotive lights (F0)
    /// loco.set_function(0, 1).await?;
    ///
    /// // Toggle the horn (assuming it's on F2)
    /// loco.set_function(2, 2).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_function(&self, function_index: u8, action: u8) -> io::Result<()> {
        if function_index > 31 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Function index must be between 0 and 31",
            ));
        }

        if action > 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Action must be 0 (off), 1 (on), or 2 (toggle)",
            ));
        }

        let addr_bytes = self.addr.to_be_bytes();
        let addr_msb = if self.addr >= 128 {
            0xC0 | addr_bytes[0]
        } else {
            addr_bytes[0]
        };

        // Create the function byte (TTNNNNNN): TT is action type, NNNNNN is function index
        let function_byte = (action << 6) | (function_index & 0x3F);

        let dbs = vec![0xF8, addr_msb, addr_bytes[1], function_byte];
        let function_msg = XBusMessage::new_dbs_vec(XBUS_LOCO_FUNCTION, dbs);

        self.station
            .send_xbus_command(function_msg, Some(XBUS_LOCO_INFO))
            .await?;

        Ok(())
    }

    /// Turns on a specific locomotive function.
    ///
    /// This is a convenience method that calls `set_function()` with the ON action.
    ///
    /// # Arguments
    ///
    /// * `function_index` - The function number (0-31) where 0 represents F0 (typically lights)
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Turn on the locomotive lights (F0)
    /// loco.function_on(0).await?;
    ///
    /// // Activate the horn (assuming it's on F2)
    /// loco.function_on(2).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn function_on(&self, function_index: u8) -> io::Result<()> {
        self.set_function(function_index, FUNC_ON).await
    }

    /// Turns off a specific locomotive function.
    ///
    /// This is a convenience method that calls `set_function()` with the OFF action.
    ///
    /// # Arguments
    ///
    /// * `function_index` - The function number (0-31) where 0 represents F0 (typically lights)
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Turn off the locomotive lights (F0)
    /// loco.function_off(0).await?;
    ///
    /// // Deactivate the horn (assuming it's on F2)
    /// loco.function_off(2).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn function_off(&self, function_index: u8) -> io::Result<()> {
        self.set_function(function_index, FUNC_OFF).await
    }

    /// Toggles a specific locomotive function (if on, turns off; if off, turns on).
    ///
    /// This is a convenience method that calls `set_function()` with the TOGGLE action.
    ///
    /// # Arguments
    ///
    /// * `function_index` - The function number (0-31) where 0 represents F0 (typically lights)
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Toggle the locomotive lights (F0)
    /// loco.function_toggle(0).await?;
    ///
    /// // Toggle the horn (assuming it's on F2)
    /// loco.function_toggle(2).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn function_toggle(&self, function_index: u8) -> io::Result<()> {
        self.set_function(function_index, FUNC_TOGGLE).await
    }

    /// Convenience method to control the locomotive's headlights (F0).
    ///
    /// This method simplifies controlling the locomotive's headlights,
    /// which are typically mapped to function F0 in DCC decoders.
    ///
    /// # Arguments
    ///
    /// * `on` - Whether to turn the lights on (true) or off (false)
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send, or Z21 does not respond.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn example(loco: &Loco) -> std::io::Result<()> {
    /// // Turn on the locomotive headlights
    /// loco.set_headlights(true).await?;
    ///
    /// // Turn off the locomotive headlights
    /// loco.set_headlights(false).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_headlights(&self, on: bool) -> io::Result<()> {
        if on {
            self.function_on(0).await
        } else {
            self.function_off(0).await
        }
    }
}
