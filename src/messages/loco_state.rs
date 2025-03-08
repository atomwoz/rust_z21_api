use tokio::io;

use super::XBusMessage;

#[derive(Clone, Copy, Debug)]
pub enum DccThrottleSteps {
    Steps14 = 0x10,
    Steps28 = 0x12,
    Steps128 = 0x13,
}

#[derive(Debug, Clone)]
pub struct LocoState {
    /// Address of the locomotive.
    pub address: u16,
    /// Idicates if another X-Bus controller (like MultiMaus, or other PC) is controlling the loco.
    pub is_busy: Option<bool>,
    /// Stepping of throttle
    pub stepping: Option<DccThrottleSteps>,
    /// Speed of the locomotive.
    /// Negative values indicate reverse.
    pub speed_percentage: Option<f64>,
    /// Is in double traction mode.
    pub double_traction: Option<bool>,
    /// Is in smart search (?)
    pub smart_search: Option<bool>,
    /// Functions flag, at index 0 is F0, at index 1 is F1, etc.
    pub functions: Option<[bool; 32]>,
}
impl TryFrom<&[u8]> for LocoState {
    type Error = io::Error;

    /// Attempts to parse a `LocoState` from a n byte slice
    ///
    /// # Errors
    ///
    /// Returns an error if the provided slice is not exactly 16 bytes long.
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from(&XBusMessage::try_from(data)?)
    }
}

impl TryFrom<&XBusMessage> for LocoState {
    type Error = io::Error;

    /// Attempts to parse a `LocoState` from a n byte slice
    ///
    /// # Errors
    ///
    /// Returns an error if the provided slice is not exactly 16 bytes long.
    fn try_from(data: &XBusMessage) -> Result<Self, Self::Error> {
        let data = data.get_dbs();
        let len = data.len();
        if len <= 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid LocoState data length",
            ));
        }
        // The two highest bits in Adr_MSB must be ignored
        let addr = [data[0] & 0b00111111, data[1]];
        let address = u16::from_be_bytes(addr);
        let mut is_busy = None;
        let mut stepping = None;
        let mut speed_percentage = None;
        let mut double_traction = None;
        let mut smart_search = None;
        let mut functions = None;

        if len >= 3 {
            is_busy = Some((data[2] & 0b0000_1000) != 0);
            const STEPPING_MASK: u8 = 0b0000_0111;
            stepping = if (data[2] & STEPPING_MASK) == 0 {
                Some(DccThrottleSteps::Steps14)
            } else if (data[2] & STEPPING_MASK) == 2 {
                Some(DccThrottleSteps::Steps28)
            } else if (data[2] & STEPPING_MASK) == 4 {
                Some(DccThrottleSteps::Steps128)
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid coding of DCC Stepping",
                ));
            };
        }
        if len >= 4 {
            // To further speed calculation:
            let is_going_forward = data[3] & 0b1000_0000 != 0;

            let speed = (data[3] & 0b0111_1111) as f64;
            let speed =
                match stepping.expect("That could not happen err_code: DCC Stepping is NULL") {
                    DccThrottleSteps::Steps14 => speed / 14.,
                    DccThrottleSteps::Steps28 => speed / 28.,
                    DccThrottleSteps::Steps128 => speed / 128.,
                };

            speed_percentage = Some((if is_going_forward { speed } else { -speed }) * 100.);
        }
        if len >= 5 {
            double_traction = Some(data[4] & 0b01000000 != 0);
            smart_search = Some(data[4] & 0b00100000 != 0);
            let mut functions_array = [false; 32];
            functions_array[0] = data[4] & 0b0001_0000 != 0;
            functions_array[4] = data[4] & 0b0000_1000 != 0;
            functions_array[3] = data[4] & 0b0000_0100 != 0;
            functions_array[2] = data[4] & 0b0000_0010 != 0;
            functions_array[1] = data[4] & 0b0000_0001 != 0;

            if len >= 6 {
                for i in 0..8 {
                    functions_array[i + 5] = data[5] & (1 << i) != 0;
                }
            }
            if len >= 7 {
                for i in 0..8 {
                    functions_array[i + 13] = data[6] & (1 << i) != 0;
                }
            }
            if len >= 8 {
                for i in 0..8 {
                    functions_array[i + 21] = data[7] & (1 << i) != 0;
                }
            }
            if len >= 9 {
                for i in 0..3 {
                    functions_array[i + 29] = data[8] & (1 << i) != 0;
                }
            }

            functions = Some(functions_array);
        }

        Ok(LocoState {
            address,
            is_busy,
            stepping,
            speed_percentage,
            double_traction,
            smart_search,
            functions,
        })
    }
}
