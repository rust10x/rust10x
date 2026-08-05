//! Application layer: business protocol, service logic, and typed client.
//!
//! Nothing here knows about framing or correlation ids, those belong to the
//! `ipc::socket` transport.

// region:    --- Modules

mod client;
mod contract;
mod server;

pub use client::*;
pub use contract::*;
pub use server::*;

// endregion: --- Modules
