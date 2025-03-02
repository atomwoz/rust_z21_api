#[derive(Debug)]
pub struct Packet {
    data_len: u16,
    header: u16,
    data: Vec<u8>,
}

impl Packet {
    //Panicks when data length is greater than 65535
    pub fn with_header_and_data(header: u16, data: &[u8]) -> Packet {
        if data.len() + 4 > u16::MAX as usize {
            panic!("Packet payload is too big");
        }

        let calculated_len = (data.len() + 4) as u16;
        Packet {
            header: header,
            data_len: calculated_len,
            data: Vec::from(data),
        }
    }
    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
    pub fn get_header(&self) -> u16 {
        self.header
    }
    pub fn get_data_len(&self) -> u16 {
        self.data_len
    }
}

impl Into<Vec<u8>> for Packet {
    fn into(self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend(&self.data_len.to_le_bytes());
        result.extend(&self.header.to_le_bytes());
        result.extend(&self.data);
        result
    }
}

impl From<Vec<u8>> for Packet {
    fn from(data: Vec<u8>) -> Packet {
        let data_len = u16::from_le_bytes([data[0], data[1]]);
        let header = u16::from_le_bytes([data[2], data[3]]);
        let payload = data[4..].to_vec();
        Packet {
            data_len: data_len,
            header: header,
            data: payload,
        }
    }
}
