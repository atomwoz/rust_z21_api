use std::net::UdpSocket;

use z21_api::Z21Station;

fn main() {
    let sock = UdpSocket::bind("0.0.0.0:0").expect("Can't bind auto address");
    sock.connect("192.168.0.111:21105").unwrap();
    let station = Z21Station::from_socket(sock);
    println!("Serial number: {}", station.get_serial_number());
    station.voltage_off();
}
