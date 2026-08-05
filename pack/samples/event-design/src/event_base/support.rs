//! Internal error translation shared by channel endpoint wrappers.

use crate::event_base::event_base_error::{EventBaseError, EventBaseResult};
use crossfire::{TryRecvError, TrySendError};

pub(super) fn handle_send_result<E>(
	result: core::result::Result<(), E>,
	name: &'static str,
) -> EventBaseResult<()> {
	result.map_err(|_| EventBaseError::TxDisconnected { name })
}

pub(super) fn handle_try_send_result<T>(
	result: core::result::Result<(), TrySendError<T>>,
	name: &'static str,
) -> EventBaseResult<Option<T>> {
	match result {
		Ok(()) => Ok(None),
		Err(TrySendError::Full(message)) => Ok(Some(message)),
		Err(TrySendError::Disconnected(_)) => Err(EventBaseError::TxDisconnected { name }),
	}
}

pub(super) fn handle_recv_result<T, E>(
	result: core::result::Result<T, E>,
	name: &'static str,
) -> EventBaseResult<T> {
	result.map_err(|_| EventBaseError::RxDisconnected { name })
}

pub(super) fn handle_try_recv_error<T>(error: TryRecvError, name: &'static str) -> EventBaseResult<Option<T>> {
	match error {
		TryRecvError::Empty => Ok(None),
		TryRecvError::Disconnected => Err(EventBaseError::RxDisconnected { name }),
	}
}
