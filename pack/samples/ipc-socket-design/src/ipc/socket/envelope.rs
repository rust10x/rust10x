//! Transport envelope: correlation id and the generic request/response frames.
//!
//! The envelope is plumbing only, the application supplies the method and reply
//! payload types.

use derive_more::{Deref, Display, From, Into};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// First id handed out by `RequestIdGen`.
///
/// `0` stays reserved as a future no-correlation sentinel, for example for a
/// fire-and-forget `Notification` frame.
const FIRST_REQUEST_ID: u64 = 1;

// region:    --- Types

/// Correlation id, unique among the in-flight requests of one client connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Deref, Into, From)]
#[display("{_0}")]
pub struct RequestId(u64);

/// Per-connection monotonic request id generator.
#[derive(Debug)]
pub struct RequestIdGen(AtomicU64);

/// Client to service message, `M` is the application method payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request<M> {
	pub id: RequestId,
	pub method: M,
}

/// Service to client message, `R` is the application reply payload.
///
/// Only responses travel this way for now. A `Notification` message can be added
/// later for server push, without changing the request path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<R> {
	/// Echoed from the request, the service never generates ids.
	pub id: RequestId,

	pub reply: R,
}

// endregion: --- Types

impl RequestIdGen {
	/// Returns the next id for this connection.
	pub fn next_id(&self) -> RequestId {
		RequestId(self.0.fetch_add(1, Ordering::Relaxed))
	}
}

impl Default for RequestIdGen {
	fn default() -> Self {
		Self(AtomicU64::new(FIRST_REQUEST_ID))
	}
}
