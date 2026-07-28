//! Application-owned channel facade over Crossfire.
//!
//! This is the Level 1 messaging layer of the event design. Level 2 use-case
//! aliases and Level 3 domain types are defined in their own domain modules
//! and build on the endpoints exposed here.
//!
//! This layer exposes named, bounded MPSC, MPMC, SPSC, synchronous SPSC, and
//! one-shot channels without leaking Crossfire endpoint types. Operations
//! translate peer disconnections into [`EventBaseError`].

// region:    --- Modules

mod support;

mod common;
mod event_base_error;
mod event_mpmc;
mod event_mpsc;
mod event_once;
mod event_spsc;

pub use common::*;
pub use event_base_error::{EventBaseError, EventBaseResult};
pub use event_mpmc::*;
pub use event_mpsc::*;
pub use event_once::*;
pub use event_spsc::*;

// endregion: --- Modules
