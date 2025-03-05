//! # z21_async
//!
//! This crate provides asynchronous communication with a Roco Fleischmann Z21 station.
//! It implements a reusable approach for sending and receiving asynchronous UDP packets to and from the Z21 station.
//! The crate is based on the Tokio runtime.
//!
//! ## Features
//! - Asynchronous UDP communication using Tokio.
//! - Loco and peripheral control.
//! - CV programming.
//! - Dedicated event subscription for system state changes.
//! - Well-documented and reusable API for integration into other projects.

mod packet;
mod station;
pub use station::Loco;
pub use station::Z21Station;
pub mod messages;
