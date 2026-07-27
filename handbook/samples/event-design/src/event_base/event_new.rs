//! Constructors for the event-base channel endpoint wrappers.

use crate::event_base::event_base_error::{EventBaseError, EventBaseResult};
use crate::event_base::{MpmcRx, MpmcTx, MpscRx, MpscTx, OnceRx, OnceTx, SpscRx, SpscTx};
use crossfire::{mpmc, mpsc, oneshot, spsc};

/// Default capacity used by bounded channel constructors.
pub const DEFAULT_CAPACITY: usize = 1000;

// region:    --- Factory Functions

/// Creates a bounded asynchronous MPSC channel with [`DEFAULT_CAPACITY`].
///
/// `name` is retained by both endpoints for diagnostics and disconnection
/// errors.
pub fn new_mpsc_bounded_default<T>(name: &'static str) -> EventBaseResult<(MpscTx<T>, MpscRx<T>)>
where
	T: Send + 'static,
{
	new_mpsc_bounded(name, DEFAULT_CAPACITY)
}

/// Creates a bounded asynchronous MPSC channel.
///
/// `capacity` is the number of queued messages and must be greater than zero.
/// A zero capacity returns [`EventBaseError::InvalidCapacity`].
pub fn new_mpsc_bounded<T>(name: &'static str, capacity: usize) -> EventBaseResult<(MpscTx<T>, MpscRx<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(EventBaseError::InvalidCapacity { name, capacity });
	}
	let (tx, rx) = mpsc::bounded_async::<T>(capacity);
	Ok((MpscTx { inner: tx, name }, MpscRx { inner: rx, name }))
}

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

/// Creates a single-use asynchronous channel.
///
/// `name` is retained by both endpoints for diagnostics and disconnection
/// errors.
pub fn new_once<T>(name: &'static str) -> (OnceTx<T>, OnceRx<T>) {
	let (tx, rx) = oneshot::oneshot();
	(OnceTx { inner: tx, name }, OnceRx { inner: rx, name })
}

// endregion: --- Factory Functions
