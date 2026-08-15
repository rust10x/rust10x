//! Single-use event-base channel endpoints.

use crate::event_base::event_base_error::{EventBaseError, EventBaseResult};
use crossfire::{TryRecvError, oneshot};

// region:    --- Factory Functions

/// Creates a single-use asynchronous channel.
///
/// `name` is retained by both endpoints for diagnostics and disconnection
/// errors.
pub fn new_once<T>(name: &'static str) -> (OnceTx<T>, OnceRx<T>) {
	let (tx, rx) = oneshot::oneshot();
	(OnceTx { inner: tx, name }, OnceRx { inner: rx, name })
}

// endregion: --- Factory Functions

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

	/// Returns whether the receiver has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
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

	/// Returns whether the receiver has disconnected.
	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

// endregion: --- Implementation OnceRx<T>

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[tokio::test]
	async fn test_event_base_once_send_recv() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, rx) = new_once::<u32>("once-test");

		// -- Exec
		tx.send(11);

		// -- Check
		assert_eq!(rx.name(), "once-test");
		let value = rx.recv().await?;
		assert_eq!(value, 11);
		Ok(())
	}

	#[test]
	fn test_event_base_once_try_recv_empty() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, mut rx) = new_once::<u32>("once-empty-test");

		// -- Exec
		let value = rx.try_recv()?;

		// -- Check
		assert_eq!(value, None);
		assert!(!tx.is_disconnected());
		Ok(())
	}

	#[tokio::test]
	async fn test_event_base_once_disconnection() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, rx) = new_once::<u32>("once-disconnect-test");
		drop(tx);

		// -- Exec
		let error = rx.recv().await;

		// -- Check
		assert!(matches!(
			error,
			Err(EventBaseError::RxDisconnected {
				name: "once-disconnect-test"
			})
		));
		Ok(())
	}
}

// endregion: --- Tests
