use std::net::UdpSocket;

use z21_packet::Packet;

mod z21_packet;

const DEFAULT_TIMEOUT: u64 = 3000;

pub struct Z21Station {
    socket: UdpSocket,
    timeout_ms: u64,
}
impl Z21Station {
    pub fn from_socket(socket: UdpSocket) -> Z21Station {
        let station = Z21Station {
            socket: socket,
            timeout_ms: DEFAULT_TIMEOUT,
        };
        station.set_broadcast();
        station
    }
    fn set_broadcast(&self) {
        let packet = Packet::with_header_and_data(0x50, &u16::MAX.to_le_bytes());
        self.send_packet(packet);
    }
    fn send_packet(&self, packet: Packet) {
        let data: Vec<u8> = packet.into();
        self.socket.send(&data).unwrap();
    }
    fn receive_packet(&self) -> Packet {
        let mut buffer = [0; 1024];
        let (size, _) = self.socket.recv_from(&mut buffer).unwrap();
        let mut data = Vec::new();
        data.extend(&buffer[..size]);
        Packet::from(data)
    }
    fn recive_packet_wtih_header(&self, header: u16) -> Packet {
        loop {
            let packet = self.receive_packet();
            if packet.get_header() == header {
                return packet;
            }
        }
    }
    pub fn voltage_off(&self) {
        let packet = Packet::with_header_and_data(0x40, &[0x21, 0x80, 0xa1]);
        self.send_packet(packet);
    }
    pub fn get_serial_number(&self) -> u32 {
        let packet = Packet::with_header_and_data(0x10, &[]);
        self.send_packet(packet);
        let response = self.recive_packet_wtih_header(0x10);
        let data = response.get_data();
        u32::from_le_bytes([data[0], data[1], data[2], data[3]])
    }
    pub fn logout(&self) {
        let packet = Packet::with_header_and_data(0x30, &[]);
        self.send_packet(packet);
    }
}
impl Drop for Z21Station {
    fn drop(&mut self) {
        self.logout();
    }
}
