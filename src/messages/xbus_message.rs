use tokio::io;

pub const XBUS_HEADER: u16 = 0x40;
#[derive(Clone, Debug)]
pub struct XBusMessage {
    x_header: u8,
    dbs: Vec<u8>,
    xor: u8,
}

impl XBusMessage {
    pub fn new_only_header(x_header: u8) -> XBusMessage {
        XBusMessage {
            x_header,
            dbs: Vec::new(),
            xor: x_header,
        }
    }
    pub fn new_single(x_header: u8, db: u8) -> XBusMessage {
        let xor = x_header ^ db;
        XBusMessage {
            x_header,
            dbs: vec![db],
            xor,
        }
    }
    pub fn new_double(x_header: u8, db0: u8, db1: u8) -> XBusMessage {
        let xor = x_header ^ db0 ^ db1;
        XBusMessage {
            x_header,
            dbs: vec![db0, db1],
            xor,
        }
    }
    pub fn new_dbs_vec(x_header: u8, dbs: Vec<u8>) -> XBusMessage {
        let xor_byte = dbs.iter().fold(x_header, |acc, &x| acc ^ x);
        XBusMessage {
            x_header,
            dbs,
            xor: xor_byte,
        }
    }
    pub fn get_x_header(&self) -> u8 {
        self.x_header
    }
    pub fn get_dbs(&self) -> &Vec<u8> {
        &self.dbs
    }
    pub fn get_xor(&self) -> u8 {
        self.xor
    }
}

impl Into<Vec<u8>> for XBusMessage {
    fn into(self) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.push(self.x_header);
        vec.extend_from_slice(&self.dbs);
        vec.push(self.xor);
        vec
    }
}

impl TryFrom<&[u8]> for XBusMessage {
    type Error = io::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let counts = data.len() as i64 - 2;
        if counts >= 0 {
            let counts = counts as usize;
            let mut vec = Vec::with_capacity(counts);
            let x_header = data[0];
            let data_xor = data[counts + 1];
            vec.extend_from_slice(&data[1..=counts]);
            let calculated_xor = vec.iter().fold(x_header, |acc, x| acc ^ x);
            if data_xor != calculated_xor {
                return Err(io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "XBus message XOR is wrong",
                ));
            }
            Ok(XBusMessage {
                x_header,
                dbs: vec,
                xor: calculated_xor,
            })
        } else {
            Err(io::Error::new(
                std::io::ErrorKind::InvalidData,
                "XBus message is too short",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_only_header() {
        let msg = XBusMessage::new_only_header(0x21);
        assert_eq!(msg.get_x_header(), 0x21);
        assert_eq!(msg.get_dbs().len(), 0);
        assert_eq!(msg.get_xor(), 0x21);
    }

    #[test]
    fn test_new_single() {
        let msg = XBusMessage::new_single(0x21, 0x34);
        assert_eq!(msg.get_x_header(), 0x21);
        assert_eq!(msg.get_dbs(), &vec![0x34]);
        assert_eq!(msg.get_xor(), 0x21 ^ 0x34);
    }

    #[test]
    fn test_new_double() {
        let msg = XBusMessage::new_double(0x21, 0x34, 0x56);
        assert_eq!(msg.get_x_header(), 0x21);
        assert_eq!(msg.get_dbs(), &vec![0x34, 0x56]);
        assert_eq!(msg.get_xor(), 0x21 ^ 0x34 ^ 0x56);
    }

    #[test]
    fn test_new_dbs_vec() {
        let dbs = vec![0x34, 0x56, 0x78];
        let msg = XBusMessage::new_dbs_vec(0x21, dbs.clone());
        assert_eq!(msg.get_x_header(), 0x21);
        assert_eq!(msg.get_dbs(), &dbs);
        assert_eq!(msg.get_xor(), 0x21 ^ 0x34 ^ 0x56 ^ 0x78);
    }

    #[test]
    fn test_into_vec() {
        let msg = XBusMessage::new_double(0x21, 0x34, 0x56);
        let vec: Vec<u8> = msg.into();
        assert_eq!(vec, vec![0x21, 0x34, 0x56, 0x21 ^ 0x34 ^ 0x56]);
    }

    #[test]
    fn test_try_from_valid() {
        let data = vec![0x61, 0x01, 0x61 ^ 0x01];
        let msg = XBusMessage::try_from(data.as_slice()).unwrap();
        assert_eq!(msg.get_x_header(), 0x61);
        assert_eq!(msg.get_dbs(), &vec![0x01]);
        assert_eq!(msg.get_xor(), 0x61 ^ 0x01);
    }

    #[test]
    fn test_try_from_invalid_xor() {
        let data = vec![0x21, 0x34, 0x56, 0xFF]; // Wrong XOR
        let result = XBusMessage::try_from(data.as_slice());
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_too_short() {
        let data = vec![0x21]; // Too short
        let result = XBusMessage::try_from(data.as_slice());
        assert!(result.is_err());
    }
}
