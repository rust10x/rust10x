//! Asynchronous and synchronous single-producer channel endpoint wrappers.

use crate::event_base::event_base_error::EventBaseResult;
use crate::event_base::{DEFAULT_CAPACITY, EventBaseError, support};
use crossfire::{AsyncRx, AsyncTx, Rx, TryRecvError, Tx, spsc};

// region:    --- Factories

/// Creates a bounded asynchronous SPSC channel with [`DEFAULT_CAPACITY`].
///
/// `name` is retained by both endpoints for diagnostics and disconnection
/// errors.
pub fn new_spsc_bounded_default<T>(name: &'static str) -> EventBaseResult<(SpscTx<T>, SpscRx<T>)>
where
	T: Send + 'static,
{
	new_spsc_bounded(name, DEFAULT_CAPACITY)
}

/// Creates a bounded asynchronous SPSC channel.
///
/// `capacity` is the number of queued messages and must be greater than zero.
/// A zero capacity returns [`EventBaseError::InvalidCapacity`].
pub fn new_spsc_bounded<T>(name: &'static str, capacity: usize) -> EventBaseResult<(SpscTx<T>, SpscRx<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(EventBaseError::InvalidCapacity { name, capacity });
	}
	let (tx, rx) = spsc::bounded_async::<T>(capacity);
	Ok((SpscTx { inner: tx, name }, SpscRx { inner: rx, name }))
}

// endregion: --- Factories

// region:    --- SpscTx Implementations

/// SpSc SingleProducer sender. Not clonable.
pub struct SpscTx<T: Send + 'static> {
	pub(super) inner: AsyncTx<spsc::Array<T>>,
	pub(super) name: &'static str,
}

impl<T: Send + 'static> std::fmt::Debug for SpscTx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SpscTx").field("name", &self.name).finish()
	}
}

impl<T> SpscTx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Sends a message asynchronously, waiting until channel capacity is available.
	///
	/// `T: Unpin` is required by Crossfire's asynchronous send future. Callers that
	/// need to send a `!Unpin` value can use a movable pinned owner such as `Pin<Box<T>>`
	/// as the channel payload type.
	///
	/// If the receiver disconnects, returns an error without recovering
	/// `message`. Cancelling the returned future before it completes leaves
	/// delivery unspecified.
	pub async fn send(&self, message: T) -> EventBaseResult<()>
	where
		T: Unpin,
	{
		support::handle_send_result(self.inner.send(message).await, self.name)
	}

	/// Attempts to send without blocking, returning the message when the channel is full.
	///
	/// A disconnected receiver returns an error without recovering the message.
	pub fn try_send(&self, message: T) -> EventBaseResult<Option<T>> {
		support::handle_try_send_result(self.inner.try_send(message), self.name)
	}

	/// Returns whether the receiver has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}

	/// Converts this unique asynchronous sender into its synchronous counterpart.
	pub fn into_sync_tx(self) -> SyncSpscTx<T> {
		let sync_tx = self.inner.into_blocking();
		SyncSpscTx {
			inner: sync_tx,
			name: self.name,
		}
	}
}

// endregion: --- SpscTx Implementations

// region:    --- SpscRx Implementations

/// SpSc SingleConsumer receiver. Not clonable.
pub struct SpscRx<T: Send + 'static> {
	pub(super) inner: AsyncRx<spsc::Array<T>>,
	pub(super) name: &'static str,
}

impl<T: Send + 'static> std::fmt::Debug for SpscRx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SpscRx").field("name", &self.name).finish()
	}
}

impl<T> SpscRx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Mutable access keeps the receive future `Send` without requiring this single-consumer receiver to be `Sync`.
	///
	/// Returns an error after the sender disconnects and no queued message
	/// remains.
	pub async fn recv(&mut self) -> EventBaseResult<T> {
		support::handle_recv_result(self.inner.recv().await, self.name)
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	///
	/// Returns an error when the sender has disconnected.
	pub fn try_recv(&self) -> EventBaseResult<Option<T>> {
		match self.inner.try_recv() {
			// A message was immediately available.
			Ok(value) => Ok(Some(value)),

			// An empty, connected channel may receive a message later.
			Err(error @ TryRecvError::Empty) => support::handle_try_recv_error(error, self.name),

			// No message can arrive after the sender disconnects.
			Err(error @ TryRecvError::Disconnected) => support::handle_try_recv_error(error, self.name),
		}
	}

	/// Returns whether the sender has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}

	/// Converts this unique asynchronous receiver into its synchronous counterpart.
	pub fn into_sync_rx(self) -> SyncSpscRx<T> {
		let sync_rx = self.inner.into_blocking();
		SyncSpscRx {
			inner: sync_rx,
			name: self.name,
		}
	}
}

// endregion: --- SpscRx Implementations

// region:    --- SyncSpscTx Implementations

/// SpSc synchronous single-producer sender. Not clonable.
pub struct SyncSpscTx<T: Send + 'static> {
	pub(super) inner: Tx<spsc::Array<T>>,
	pub(super) name: &'static str,
}

impl<T: Send + 'static> std::fmt::Debug for SyncSpscTx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SyncSpscTx").field("name", &self.name).finish()
	}
}

impl<T> SyncSpscTx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Sends a message, blocking the current thread until channel capacity is available.
	///
	/// Do not call this where blocking prevents the receiver from making progress.
	/// A disconnected receiver returns an error without recovering the message.
	pub fn send_sync(&self, message: T) -> EventBaseResult<()> {
		support::handle_send_result(self.inner.send(message), self.name)
	}

	/// Attempts to send without blocking, returning the message when the channel is full.
	///
	/// A disconnected receiver returns an error without recovering the message.
	pub fn try_send(&self, message: T) -> EventBaseResult<Option<T>> {
		support::handle_try_send_result(self.inner.try_send(message), self.name)
	}

	/// Returns whether the receiver has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}
}

// endregion: --- SyncSpscTx Implementations

// region:    --- SyncSpscRx Implementations

/// SpSc synchronous single-consumer receiver. Not clonable.
pub struct SyncSpscRx<T: Send + 'static> {
	pub(super) inner: Rx<spsc::Array<T>>,
	pub(super) name: &'static str,
}

impl<T: Send + 'static> std::fmt::Debug for SyncSpscRx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SyncSpscRx").field("name", &self.name).finish()
	}
}

impl<T> SyncSpscRx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Receives a message, blocking the current thread until one is available.
	///
	/// Returns an error after the sender disconnects and no queued message
	/// remains.
	pub fn recv_sync(&self) -> EventBaseResult<T> {
		support::handle_recv_result(self.inner.recv(), self.name)
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	///
	/// Returns an error when the sender has disconnected.
	pub fn try_recv(&self) -> EventBaseResult<Option<T>> {
		match self.inner.try_recv() {
			// A message was immediately available.
			Ok(value) => Ok(Some(value)),

			// An empty, connected channel may receive a message later.
			Err(error @ TryRecvError::Empty) => support::handle_try_recv_error(error, self.name),

			// No message can arrive after the sender disconnects.
			Err(error @ TryRecvError::Disconnected) => support::handle_try_recv_error(error, self.name),
		}
	}

	/// Returns whether the sender has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}
}

// endregion: --- SyncSpscRx Implementations

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[tokio::test]
	async fn test_event_base_spsc_async_send_recv() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, mut rx) = new_spsc_bounded::<u32>("spsc-async-test", 1)?;

		// -- Exec
		tx.send(7).await?;
		let value = rx.recv().await?;

		// -- Check
		assert_eq!(value, 7);
		assert_eq!(tx.name(), "spsc-async-test");
		assert_eq!(rx.name(), "spsc-async-test");
		Ok(())
	}

	#[test]
	fn test_event_base_spsc_sync_conversion() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, rx) = new_spsc_bounded::<u32>("spsc-sync-test", 1)?;
		let sync_tx = tx.into_sync_tx();
		let sync_rx = rx.into_sync_rx();

		// -- Exec
		sync_tx.send_sync(9)?;
		let value = sync_rx.recv_sync()?;

		// -- Check
		assert_eq!(value, 9);
		assert_eq!(sync_tx.name(), "spsc-sync-test");
		assert_eq!(sync_rx.name(), "spsc-sync-test");
		Ok(())
	}

	#[test]
	fn test_event_base_spsc_try_operations() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, rx) = new_spsc_bounded::<u32>("spsc-try-test", 1)?;

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
	async fn test_event_base_spsc_disconnection() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, mut rx) = new_spsc_bounded::<u32>("spsc-disconnect-test", 1)?;
		drop(tx);

		// -- Exec
		let error = rx.recv().await;

		// -- Check
		assert!(matches!(
			error,
			Err(EventBaseError::RxDisconnected {
				name: "spsc-disconnect-test"
			})
		));
		assert!(rx.is_disconnected());
		Ok(())
	}
}

// endregion: --- Tests
