//! Service-side application seam.
//!
//! The socket layer moves frames and matches responses to requests, the handler
//! executes the application methods and owns the application state.

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::future::Future;

/// Application logic invoked for each decoded request frame.
///
/// Implementors are shared across every connection, so state must be interior
/// mutable, for example behind a mutex or an actor task.
pub trait RequestHandler: Send + Sync + 'static {
	/// Application method payload, decoded from the request frame.
	type Method: DeserializeOwned + Debug + Send + 'static;

	/// Application reply payload, encoded into the response frame.
	///
	/// `Sync` is required because the per-connection writer task serializes the
	/// response by reference, so the borrow lives across an await point inside a
	/// spawned task.
	type Reply: Serialize + Send + Sync + 'static;

	/// Executes one method.
	///
	/// Application level failures should travel inside `Reply` rather than as a
	/// transport error, so the connection stays usable.
	fn exec(&self, method: Self::Method) -> impl Future<Output = Self::Reply> + Send;
}
