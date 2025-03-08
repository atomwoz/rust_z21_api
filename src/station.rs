//! This module provides asynchronous communication with a RoCo Fleischmann Z21 station.
//! It implements a reusable UDP-based API that handles sending commands and receiving
//! events from the Z21 station. In particular, it supports receiving system state change
//! events (LAN_SYSTEMSTATE_DATACHANGED) through an internal broadcast channel.
//!
//! # Overview
//!
//! - Uses Tokio's asynchronous UDP socket for communication.
//! - Wraps incoming messages in a [`Packet`] structure, and propagates them via a broadcast channel.
//! - Provides helper methods for specific commands, e.g., turning voltage off, retrieving serial numbers,
//!   and logging out.
//! - The UDP socket is wrapped in an `Arc` for safe sharing across asynchronous tasks.

use crate::messages::{self, SystemState, XBusMessage};
use crate::packet::Packet;
use std::convert::TryFrom;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::time::{self, timeout};

mod loco;
pub use loco::Loco;

/// The header value for the LAN_SYSTEMSTATE_DATACHANGED event.
const LAN_SYSTEMSTATE_DATACHANGED: u16 = 0x84;
const LAN_SET_BROADCASTFLAGS: u16 = 0x50;
const LAN_SYSTEMSTATE_GETDATA: u16 = 0x85;
const X_SET_TRACK_POWER_OFF: (u8, u8) = (0x21, 0x80);
const X_SET_TRACK_POWER_ON: (u8, u8) = (0x21, 0x81);
const X_BC_TRACK_POWER: u8 = 0x61;

/// Default timeout in milliseconds for awaiting responses.
const DEFAULT_TIMEOUT_MS: u64 = 2000;

/// Default broadcast flags for the Z21 station.(Default is ONLY LOCO_INFO & TURNOUT_INFO)
const DEFAULT_BROADCAST_FLAGS: u32 = 0x00000001;

/// Represents an asynchronous connection to a Z21 station.
///
/// The `Z21Station` manages a UDP socket for communication with a Z21 station. It spawns a
/// background task to continuously listen for incoming packets and proceed these packets
/// over an internal logic.
pub struct Z21Station {
    socket: Arc<UdpSocket>,
    message_sender: broadcast::Sender<Packet>,
    message_receiver: broadcast::Receiver<Packet>,
    timeout: Duration,
    keep_alive: Arc<AtomicBool>,
    broadcast_flags: u32,
}

impl Z21Station {
    /// Creates a new `Z21Station` instance and initializes the connection to the specified
    /// Z21 station address.
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - The target address of the Z21 station (default is `"192.168.0.111:21105"`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if any step of the initialization fails, such as binding the socket,
    /// enabling broadcast, or connecting to the target address.
    pub async fn new(bind_addr: &str) -> io::Result<Self> {
        // Bind the socket to an available local port on all interfaces.
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        // Enable broadcast on the socket to allow sending messages to a broadcast address.
        socket.set_broadcast(true)?;
        // Connect the socket to the Z21 station address.
        socket.connect(bind_addr).await?;
        let socket = Arc::new(socket);

        // Create a broadcast channel for propagating incoming packets.
        let (tx, rx) = broadcast::channel(100);
        let station = Z21Station {
            socket,
            message_sender: tx,
            message_receiver: rx,
            keep_alive: Arc::new(AtomicBool::new(true)),
            broadcast_flags: DEFAULT_BROADCAST_FLAGS,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        };
        // Start the background receiver task.
        station.start_receiver();

        // Perform the initial handshake with the Z21 station.
        let result = station.initial_handshake().await;
        if let Err(e) = result {
            eprintln!(
                "There is no connection to the Z21 station, on the specified address: {}",
                bind_addr
            );
            return Err(e);
        }

        // Start the keep-alive thread.
        station.start_keep_alive_setup_broadcast_task();
        Ok(station)
    }

    /// Starts a background asynchronous task that continuously listens for incoming UDP packets.
    ///
    /// The task reads data from the socket, converts it into a [`Packet`], and then sends it through
    /// the internal broadcast channel so that subscribers can process the packet.
    fn start_receiver(&self) {
        let socket = Arc::clone(&self.socket);
        let message_sender = self.message_sender.clone();

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                match socket.recv(&mut buf).await {
                    Ok(size) => {
                        // Copy the received data into a vector.
                        let data = buf[..size].to_vec();
                        // Convert the raw data into a Packet.
                        let packet = Packet::from(data);
                        //println!("Received packet with header: {:?}", packet.get_header());
                        // if packet.get_header() == 64 {
                        //     let xbus_msg = XBusMessage::try_from(
                        //         &packet.get_data()[0..packet.get_data_len() as usize - 4],
                        //     );
                        //     if let Ok(msg) = xbus_msg {
                        //         println!(
                        //             "Received XBus message with header: {:02x}",
                        //             msg.get_x_header()
                        //         );
                        //     } else {
                        //         eprintln!("Failed to parse XBus message");
                        //     }
                        // }

                        // Broadcast the packet to all subscribers.
                        if let Err(e) = message_sender.send(packet) {
                            eprintln!("Failed to send packet via broadcast channel: {:?}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving packet: {:?}", e);
                        break;
                    }
                }
            }
        });
    }

    async fn initial_handshake(&self) -> io::Result<()> {
        let packet = Packet::with_header_and_data(LAN_SYSTEMSTATE_GETDATA, &[]);
        self.send_packet(packet).await?;
        let _ = self
            .receive_packet_with_header(LAN_SYSTEMSTATE_DATACHANGED)
            .await?;
        Ok(())
    }

    async fn send_set_broadcast_flags(socket: &Arc<UdpSocket>, flags: u32) -> io::Result<()> {
        let flags = flags.to_le_bytes();
        let broadcast_packet = Packet::with_header_and_data(LAN_SET_BROADCASTFLAGS, &flags);
        let broadcast_packet: Vec<_> = broadcast_packet.into();
        socket.send(&broadcast_packet).await?;
        Ok(())
    }

    /// Keeps connection alive by sending a broadcast packet to the Z21 station.
    fn start_keep_alive_setup_broadcast_task(&self) {
        let socket = Arc::clone(&self.socket);
        let flags = self.broadcast_flags;
        let keep_alive = Arc::clone(&self.keep_alive);
        tokio::spawn(async move {
            loop {
                let _result = Self::send_set_broadcast_flags(&socket, flags).await;
                tokio::time::sleep(Duration::from_secs(10)).await;

                if !keep_alive.load(Ordering::Relaxed) {
                    break;
                }
            }
        });
    }

    /// Sends a [`Packet`] asynchronously to the connected Z21 station.
    ///
    /// The packet is serialized into a byte vector and sent through the UDP socket.
    ///
    /// # Arguments
    ///
    /// * `packet` - The [`Packet`] to be transmitted.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send.
    async fn send_packet(&self, packet: Packet) -> io::Result<()> {
        let data: Vec<u8> = packet.into();
        // Send the serialized packet through the connected UDP socket.
        self.socket.send(&data).await?;
        Ok(())
    }
    async fn send_packet_external(socket: &Arc<UdpSocket>, packet: Packet) -> io::Result<()> {
        let data: Vec<u8> = packet.into();
        // Send the serialized packet through the connected UDP socket.
        socket.send(&data).await?;
        Ok(())
    }

    /// Sends an XBus packet without waiting for a response
    ///
    /// # Arguments
    ///
    /// * `xbus_message` - The XBus message to send
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the packet fails to send
    async fn send_xbus_packet(&self, xbus_message: XBusMessage) -> io::Result<()> {
        let data: Vec<u8> = xbus_message.into();
        let packet = Packet::with_header_and_data(messages::XBUS_HEADER, &data);
        self.send_packet(packet).await
    }

    /// Sends an XBus command and waits for the expected response
    ///
    /// # Arguments
    ///
    /// * `xbus_message` - The XBus message to send
    /// * `expected_response_xbus_header` - Optional expected response header. If None, uses the sent message header
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if:
    /// - The packet fails to send
    /// - No response is received within the timeout period
    /// - The response has an invalid format
    async fn send_xbus_command(
        &self,
        xbus_message: XBusMessage,
        expected_response_xbus_header: Option<u8>,
    ) -> io::Result<XBusMessage> {
        let x_header = xbus_message.get_x_header();
        self.send_xbus_packet(xbus_message).await?;

        let expected_header = expected_response_xbus_header.unwrap_or(x_header);
        let xbus_return = self.receive_xbus_packet(expected_header).await?;
        Ok(xbus_return)
    }

    /// Asynchronously waits for a packet with the specified header.
    ///
    /// This function listens on the internal broadcast channel and filters incoming packets,
    /// returning the first packet that matches the given header value.
    ///
    /// # Arguments
    ///
    /// * `header` - The header value to filter for.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the broadcast channel is closed or an error occurs while receiving.
    async fn receive_packet_with_header(&self, header: u16) -> io::Result<Packet> {
        let mut msg_rcv = self.message_receiver.resubscribe();
        match timeout(self.timeout, async {
            loop {
                match msg_rcv.recv().await {
                    Ok(packet) => {
                        if packet.get_header() == header {
                            return Ok(packet);
                        }
                    }
                    Err(_) => {
                        return Err(io::Error::new(io::ErrorKind::Other, "Channel closed"));
                    }
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("Timeout waiting for packet with header 0x{:04x}", header),
            )),
        }
    }

    async fn receive_xbus_packet(&self, expected_xbus_header: u8) -> io::Result<XBusMessage> {
        let mut msg_rcv = self.message_receiver.resubscribe();
        match timeout(self.timeout, async {
            loop {
                match msg_rcv.recv().await {
                    Ok(packet) => {
                        if packet.get_header() == messages::XBUS_HEADER {
                            let end_payload = packet.get_data_len() as isize - 4;
                            if end_payload <= 0 {
                                continue;
                            }
                            let end_payload = end_payload as usize;
                            let payload = &packet.get_data()[0..end_payload];
                            let xbus_msg = XBusMessage::try_from(payload);
                            if let Ok(msg) = xbus_msg {
                                if msg.get_x_header() == expected_xbus_header {
                                    return Ok(msg);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        return Err(io::Error::new(io::ErrorKind::Other, "Channel closed"));
                    }
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "Timeout waiting for XBus message with header 0x{:02x}",
                    expected_xbus_header
                ),
            )),
        }
    }

    /// Receives a single packet from the internal broadcast channel.
    ///
    /// This method awaits the next available packet regardless of its header.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the broadcast channel is closed.
    async fn receive_packet(&self) -> io::Result<Packet> {
        let mut msg_rcv = self.message_receiver.resubscribe();
        match timeout(self.timeout, async {
            match msg_rcv.recv().await {
                Ok(packet) => Ok(packet),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Channel closed")),
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Timeout waiting for packet",
            )),
        }
    }

    /// Turns off the track voltage - same as pressing STOP on the Z21 station / MultiMaus.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the command packet fails to send.
    pub async fn voltage_off(&self) -> io::Result<()> {
        self.send_xbus_command(
            XBusMessage::new_single(X_SET_TRACK_POWER_OFF.0, X_SET_TRACK_POWER_OFF.1),
            Some(X_BC_TRACK_POWER),
        )
        .await?;
        Ok(())
    }

    /// Turns on the track voltage - turning off programming mode, and disabling EMERG STOP from MultiMaus or button on Z21.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the command packet fails to send.
    pub async fn voltage_on(&self) -> io::Result<()> {
        self.send_xbus_command(
            XBusMessage::new_single(X_SET_TRACK_POWER_ON.0, X_SET_TRACK_POWER_ON.1),
            Some(X_BC_TRACK_POWER),
        )
        .await?;
        Ok(())
    }

    /// Retrieves the serial number from the Z21 station as 32-bit unsigned integer.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if:
    /// - Sending the request fails.
    /// - The response times out.
    /// - The response data is invalid (e.g., too short).
    pub async fn get_serial_number(&self) -> io::Result<u32> {
        let packet = Packet::with_header_and_data(0x10, &[]);
        self.send_packet(packet).await?;
        let response = self.receive_packet_with_header(0x10).await?;
        let data = response.get_data();
        if data.len() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Response data too short",
            ));
        }
        Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    pub fn subscribe_system_state(
        &self,
        freq_in_sec: f64,
        subscriber: Box<dyn Fn(SystemState) + Send + Sync>,
    ) {
        let mut receiver = self.message_receiver.resubscribe();
        let socket = Arc::clone(&self.socket);
        let keep_alive = Arc::clone(&self.keep_alive);
        let packet = Packet::with_header_and_data(LAN_SYSTEMSTATE_GETDATA, &[]);
        tokio::spawn(async move {
            loop {
                let result = Self::send_packet_external(&socket, packet.clone()).await;
                if let Err(_) = result {
                    break;
                }

                time::sleep(Duration::from_millis((1000. / freq_in_sec) as u64)).await;

                if !keep_alive.load(Ordering::Relaxed) {
                    break;
                }
            }
        });
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(packet) => {
                        if packet.get_header() == LAN_SYSTEMSTATE_DATACHANGED {
                            let state = SystemState::try_from(&packet.get_data()[..]);
                            if let Ok(state) = state {
                                subscriber(state);
                            }
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });
    }

    /// Logs out from the Z21 station by sending a logout command.
    /// It should be called at the end of the session to terminate the connection gracefully.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the logout command fails to send.
    pub async fn logout(&self) -> io::Result<()> {
        let packet = Packet::with_header_and_data(0x30, &[]);
        self.send_packet(packet).await
    }
}

impl Drop for Z21Station {
    fn drop(&mut self) {
        self.keep_alive.store(false, Ordering::Relaxed);
    }
}
