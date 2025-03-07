//! This crate provides asynchronous communication with a Roco Fleischmann Z21 station.
//! It implements a reusable approach for sending and receiving asynchronous commands to and from the Z21 station.  
//! The crate is based on the Tokio runtime.
//!
//! ## Features
//! - Interacting with system state of Z21
//! - Loco and peripheral control.
//! - CV programming.
//! - Asynchronous, subscription-based event handling.
//! - Error handling.
//! - Ready to use driver for integration into other projects.

mod packet;
mod station;
pub use station::Loco;
pub use station::Z21Station;
pub mod messages;
