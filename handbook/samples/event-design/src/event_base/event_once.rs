//! Single-use event-base channel endpoints.

use crate::event_base::event_base_error::{EventBaseError, EventBaseResult};
use crossfire::{TryRecvError, oneshot};

// region:    --- Implementation OnceTx<T>

/// Single-use producer, consumed on send.
pub struct OnceTx<T> {
	pub(super) inner: oneshot::TxOneshot<T>,
	pub(super) name: &'static str,
}

impl<T> std::fmt::Debug for OnceTx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OnceTx").field("name", &self.name).finish()
	}
}

impl<T> OnceTx<T> {
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Sends the message without delivery status because the underlying one-shot sender does not report a dropped receiver.
	pub fn send(self, message: T) {
		self.inner.send(message);
	}
}

// endregion: --- Implementation OnceTx<T>

// region:    --- Implementation OnceRx<T>

/// Single-use consumer, consumed on recv.
pub struct OnceRx<T> {
	pub(super) inner: oneshot::RxOneshot<T>,
	pub(super) name: &'static str,
}

impl<T> std::fmt::Debug for OnceRx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OnceRx").field("name", &self.name).finish()
	}
}

impl<T> OnceRx<T> {
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Waits for the single message.
	///
	/// Returns [`EventBaseError::RxDisconnected`] when the sender is dropped
	/// before sending.
	pub async fn recv(self) -> EventBaseResult<T> {
		self.inner
			.recv_async()
			.await
			.map_err(|_| EventBaseError::RxDisconnected { name: self.name })
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	pub fn try_recv(&mut self) -> EventBaseResult<Option<T>> {
		match self.inner.try_recv() {
			Ok(value) => Ok(Some(value)),
			Err(TryRecvError::Empty) => Ok(None),
			Err(TryRecvError::Disconnected) => Err(EventBaseError::RxDisconnected { name: self.name }),
		}
	}
}

// endregion: --- Implementation OnceRx<T>
