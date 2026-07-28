//! Application client: typed facade over the generic socket client.
//!
//! The socket client only knows how to send a method payload and await the
//! correlated reply, the business call surface lives here.

use super::contract::{BiParams, Call, CallResult, CounterIncrParams};
use crate::Result;
use crate::ipc::socket::ClientConnection;
use std::path::Path;

/// Socket client specialized for the application protocol.
type SocketClient = ClientConnection<Call, CallResult>;

/// Connected application client, safe to call concurrently through a shared reference.
pub struct Client {
	client: SocketClient,
}

impl Client {
	/// Connects to the service socket.
	pub async fn connect(label: impl Into<String>, socket_path: impl AsRef<Path>) -> Result<Self> {
		let client = SocketClient::connect(label, socket_path).await?;

		Ok(Self { client })
	}

	/// Accessors
	pub fn label(&self) -> &str {
		self.client.label()
	}
}

/// Calls
impl Client {
	pub async fn add(&self, a: i64, b: i64) -> Result<i64> {
		self.call_value(Call::Add(BiParams { a, b })).await
	}

	pub async fn multiply(&self, a: i64, b: i64) -> Result<i64> {
		self.call_value(Call::Multiply(BiParams { a, b })).await
	}

	pub async fn counter_incr(&self, by: i64) -> Result<i64> {
		self.call_value(Call::CounterIncr(CounterIncrParams { by })).await
	}

	pub async fn counter_get(&self) -> Result<i64> {
		self.call_value(Call::CounterGet).await
	}

	async fn call_value(&self, call: Call) -> Result<i64> {
		match self.client.invoke(call).await? {
			CallResult::Value(value) => Ok(value),
			CallResult::Error(message) => Err(format!("{} - service error: {message}", self.label()).into()),
		}
	}
}
