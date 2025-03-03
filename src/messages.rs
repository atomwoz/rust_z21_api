use tokio::io;

/// Represents the system state as reported by the Z21 station.
///
/// The structure corresponds to 16 bytes of data in the LAN_SYSTEMSTATE_DATACHANGED event.
#[derive(Debug, Clone)]
pub struct SystemState {
    /// Current on the main track in mA.
    pub main_current: i16,
    /// Current on the programming track in mA.
    pub prog_current: i16,
    /// Smoothed current on the main track in mA.
    pub filtered_main_current: i16,
    /// Command station internal temperature in °C.
    pub temperature: i16,
    /// Supply voltage in mV.
    pub supply_voltage: u16,
    /// Internal voltage (identical to track voltage) in mV.
    pub vcc_voltage: u16,
    /// Bitmask representing the central state.
    pub central_state: u8,
    /// Extended central state bitmask.
    pub central_state_ex: u8,
    /// Reserved byte.
    pub reserved: u8,
    /// Capabilities bitmask (from Z21 FW Version 1.42).
    pub capabilities: u8,
}
impl TryFrom<&[u8]> for SystemState {
    type Error = io::Error;

    /// Attempts to parse a `SystemState` from a 16-byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the provided slice is not exactly 16 bytes long.
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid SystemState data length",
            ));
        }
        Ok(SystemState {
            main_current: i16::from_le_bytes([data[0], data[1]]),
            prog_current: i16::from_le_bytes([data[2], data[3]]),
            filtered_main_current: i16::from_le_bytes([data[4], data[5]]),
            temperature: i16::from_le_bytes([data[6], data[7]]),
            supply_voltage: u16::from_le_bytes([data[8], data[9]]),
            vcc_voltage: u16::from_le_bytes([data[10], data[11]]),
            central_state: data[12],
            central_state_ex: data[13],
            reserved: data[14],
            capabilities: data[15],
        })
    }
}
impl Into<Vec<u8>> for SystemState {
    /// Converts a `SystemState` into a 16-byte vector.
    fn into(self) -> Vec<u8> {
        let mut result = Vec::with_capacity(16);
        result.extend(&self.main_current.to_le_bytes());
        result.extend(&self.prog_current.to_le_bytes());
        result.extend(&self.filtered_main_current.to_le_bytes());
        result.extend(&self.temperature.to_le_bytes());
        result.extend(&self.supply_voltage.to_le_bytes());
        result.extend(&self.vcc_voltage.to_le_bytes());
        result.push(self.central_state);
        result.push(self.central_state_ex);
        result.push(self.reserved);
        result.push(self.capabilities);
        result
    }
}
