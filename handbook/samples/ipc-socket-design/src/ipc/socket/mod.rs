//! Unix domain socket transport: length-delimited postcard frames with
//! request/response correlation over a multiplexed connection.
//!
//! This module is transport plumbing only. The application supplies the method
//! and reply payload types, plus the service logic behind `Handler`.

// region:    --- Modules

mod client_transport;
mod envelope;
mod request_handler;
mod server_transport;
mod wire;

pub use client_transport::*;
pub use envelope::*;
pub use request_handler::*;
pub use server_transport::*;

// endregion: --- Modules
