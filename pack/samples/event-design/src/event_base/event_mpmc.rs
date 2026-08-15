//! Asynchronous multi-producer channel endpoint wrappers.

use crate::event_base::event_base_error::{EventBaseError, EventBaseResult};
use crate::event_base::{DEFAULT_CAPACITY, support};
use crossfire::{MAsyncRx, MAsyncTx, TryRecvError, TrySendError, mpmc};

// region:    --- Factories

/// Creates a bounded asynchronous MPMC channel with [`DEFAULT_CAPACITY`].
///
/// `name` is retained by both endpoints for diagnostics and disconnection
/// errors.
pub fn new_mpmc_bounded_default<T>(name: &'static str) -> EventBaseResult<(MpmcTx<T>, MpmcRx<T>)>
where
	T: Send + 'static,
{
	new_mpmc_bounded(name, DEFAULT_CAPACITY)
}

/// Creates a bounded asynchronous MPMC channel.
///
/// `capacity` is the number of queued messages and must be greater than zero.
/// A zero capacity returns [`EventBaseError::InvalidCapacity`].
pub fn new_mpmc_bounded<T>(name: &'static str, capacity: usize) -> EventBaseResult<(MpmcTx<T>, MpmcRx<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(EventBaseError::InvalidCapacity { name, capacity });
	}
	let (tx, rx) = mpmc::bounded_async::<T>(capacity);
	Ok((MpmcTx { inner: tx, name }, MpmcRx { inner: rx, name }))
}

// endregion: --- Factories

// region:    --- MpmcTx Implementations

/// MpMc MultiProducer MultiConsumer sender. Clonable, allowing multiple senders.
pub struct MpmcTx<T: Send + 'static> {
	pub(super) inner: MAsyncTx<mpmc::Array<T>>,
	pub(super) name: &'static str,
}

// Implemented manually because deriving Clone can unnecessarily require T: Clone,
// while cloning a channel handle does not clone its queued messages.
impl<T: Send + 'static> Clone for MpmcTx<T> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			name: self.name,
		}
	}
}

impl<T: Send + 'static> std::fmt::Debug for MpmcTx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MpmcTx").field("name", &self.name).finish()
	}
}

impl<T> MpmcTx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Returns whether every receiver has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}

	/// Sends a message asynchronously, waiting until channel capacity is available.
	///
	/// `T: Unpin` is required by Crossfire's asynchronous send future. Callers that
	/// need to send a `!Unpin` value can use a movable pinned owner such as `Pin<Box<T>>`
	/// as the channel payload type.
	///
	/// If every receiver disconnects, returns
	/// [`EventBaseError::TxDisconnected`] and does not recover `message`.
	/// Cancelling the returned future before it completes leaves delivery
	/// unspecified.
	pub async fn send(&self, message: T) -> EventBaseResult<()>
	where
		T: Unpin,
	{
		support::handle_send_result(self.inner.send(message).await, self.name)
	}

	/// Attempts to send without blocking, returning the message when the channel is full.
	///
	/// Returns [`EventBaseError::TxDisconnected`] when every receiver has
	/// disconnected. The message is not recovered in that case.
	pub fn try_send(&self, message: T) -> EventBaseResult<Option<T>> {
		support::handle_try_send_result(self.inner.try_send(message), self.name)
	}

	/// Sends on the current thread, blocking only when the bounded channel is full.
	///
	/// The non-blocking attempt avoids converting sender modes while capacity is
	/// available. On backpressure, the recovered message is sent through a cloned
	/// blocking handle, preserving this sender for subsequent asynchronous use.
	///
	/// This blocks the current thread while the channel is full. Do not call it
	/// where blocking prevents a receiver from making progress. A disconnected
	/// receiver returns [`EventBaseError::TxDisconnected`] without recovering the
	/// message.
	pub fn send_sync(&self, message: T) -> EventBaseResult<()> {
		match self.inner.try_send(message) {
			Ok(()) => Ok(()),

			// if full, we block
			Err(TrySendError::Full(message)) => self
				.inner
				.clone()
				.into_blocking()
				.send(message)
				.map_err(|_e| EventBaseError::TxDisconnected { name: self.name }),

			Err(TrySendError::Disconnected(_)) => Err(EventBaseError::TxDisconnected { name: self.name }),
		}
	}
}

// endregion: --- MpmcTx Implementations

// region:    --- MpmcRx Implementations

/// MpMc MultiConsumer receiver. Clonable, allowing multiple consumers.
pub struct MpmcRx<T: Send + 'static> {
	pub(super) inner: MAsyncRx<mpmc::Array<T>>,
	pub(super) name: &'static str,
}

// Implemented manually because deriving Clone can unnecessarily require T: Clone,
// while cloning a channel handle does not clone its queued messages.
impl<T: Send + 'static> Clone for MpmcRx<T> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			name: self.name,
		}
	}
}

impl<T: Send + 'static> std::fmt::Debug for MpmcRx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MpmcRx").field("name", &self.name).finish()
	}
}

impl<T> MpmcRx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Returns whether every sender has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}

	/// Waits for a message.
	///
	/// Returns [`EventBaseError::RxDisconnected`] after every sender has
	/// disconnected and no queued message remains.
	pub async fn recv(&self) -> EventBaseResult<T> {
		support::handle_recv_result(self.inner.recv().await, self.name)
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	///
	/// Returns [`EventBaseError::RxDisconnected`] when no sender remains.
	pub fn try_recv(&self) -> EventBaseResult<Option<T>> {
		match self.inner.try_recv() {
			// A message was immediately available.
			Ok(value) => Ok(Some(value)),

			// An empty, connected channel may receive a message later.
			Err(error @ TryRecvError::Empty) => support::handle_try_recv_error(error, self.name),

			// No message can arrive after all senders disconnect.
			Err(error @ TryRecvError::Disconnected) => support::handle_try_recv_error(error, self.name),
		}
	}
}

// endregion: --- MpmcRx Implementations

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[tokio::test]
	async fn test_event_base_mpmc_send_recv() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, rx) = new_mpmc_bounded::<u32>("mpmc-test", 2)?;
		let second_tx = tx.clone();
		let second_rx = rx.clone();

		// -- Exec
		tx.send(1).await?;
		second_tx.send(2).await?;
		let first = rx.recv().await?;
		let second = second_rx.recv().await?;

		// -- Check
		assert_eq!([first, second], [1, 2]);
		assert_eq!(tx.name(), "mpmc-test");
		assert_eq!(rx.name(), "mpmc-test");
		Ok(())
	}

	#[tokio::test]
	async fn test_event_base_mpmc_try_operations() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, rx) = new_mpmc_bounded::<u32>("mpmc-try-test", 1)?;

		// -- Exec
		let first = tx.try_send(1)?;
		let second = tx.try_send(2)?;
		let received = rx.try_recv()?;

		// -- Check
		assert!(first.is_none());
		assert_eq!(second, Some(2));
		assert_eq!(received, Some(1));
		assert_eq!(rx.try_recv()?, None);
		Ok(())
	}

	#[tokio::test]
	async fn test_event_base_mpmc_disconnection() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, rx) = new_mpmc_bounded::<u32>("mpmc-disconnect-test", 1)?;
		drop(tx);

		// -- Exec
		let error = rx.recv().await;

		// -- Check
		assert!(matches!(
			error,
			Err(EventBaseError::RxDisconnected {
				name: "mpmc-disconnect-test"
			})
		));
		assert!(rx.is_disconnected());
		Ok(())
	}
}

// endregion: --- Tests
