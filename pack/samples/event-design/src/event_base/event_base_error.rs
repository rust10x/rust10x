//! Errors returned by event-base channel operations.

use derive_more::Display;

/// Result returned by event-base channel operations.
pub type EventBaseResult<T> = core::result::Result<T, EventBaseError>;

/// A channel configuration error or peer-disconnection signal.
///
/// A sender reports [`Self::TxDisconnected`] when no receiver remains. A
/// receiver reports [`Self::RxDisconnected`] when no sender remains.
#[non_exhaustive]
#[derive(Debug, Display)]
pub enum EventBaseError {
	/// A bounded channel was requested with zero capacity.
	#[display("invalid capacity {capacity} for channel `{name}`")]
	InvalidCapacity { name: &'static str, capacity: usize },

	/// Sending failed because every receiver has disconnected.
	#[display("channel `{name}` receiver disconnected")]
	TxDisconnected { name: &'static str },

	/// Receiving failed because every sender has disconnected.
	#[display("channel `{name}` sender disconnected")]
	RxDisconnected { name: &'static str },
}

impl EventBaseError {
	/// True when the channel peer has disconnected, which is the normal shutdown signal.
	pub fn is_disconnected(&self) -> bool {
		matches!(self, Self::TxDisconnected { .. } | Self::RxDisconnected { .. })
	}
}

impl std::error::Error for EventBaseError {}
