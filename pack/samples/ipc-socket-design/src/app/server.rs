//! Application service logic: shared counter state and the socket `Handler` impl.

use super::{BiParams, Call, CallResult, CounterIncrParams};
use crate::Result;
use crate::ipc::socket::RequestHandler;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::sleep;

/// Shared service state, observable from every client connection.
#[derive(Debug, Default)]
pub struct Server {
	counter: Mutex<i64>,
}

impl Server {
	fn counter_incr(&self, by: i64) -> Result<i64> {
		let mut counter = self.counter.lock().map_err(|_| "service - counter mutex poisoned")?;
		*counter += by;

		Ok(*counter)
	}

	fn counter_get(&self) -> Result<i64> {
		let counter = self.counter.lock().map_err(|_| "service - counter mutex poisoned")?;

		Ok(*counter)
	}
}

impl RequestHandler for Server {
	type Method = Call;
	type Reply = CallResult;

	async fn exec(&self, call: Call) -> CallResult {
		match call {
			Call::Add(BiParams { a, b }) => match a.checked_add(b) {
				Some(value) => CallResult::Value(value),
				None => CallResult::Error(format!("add overflow for {a} + {b}")),
			},
			Call::Multiply(BiParams { a, b }) => {
				// Artificial latency, so responses on one connection complete out of order.
				sleep(Duration::from_millis(40)).await;
				match a.checked_mul(b) {
					Some(value) => CallResult::Value(value),
					None => CallResult::Error(format!("multiply overflow for {a} * {b}")),
				}
			}
			Call::CounterIncr(CounterIncrParams { by }) => to_call_result(self.counter_incr(by)),
			Call::CounterGet => to_call_result(self.counter_get()),
		}
	}
}

// region:    --- Support

fn to_call_result(result: Result<i64>) -> CallResult {
	match result {
		Ok(value) => CallResult::Value(value),
		Err(err) => CallResult::Error(err.to_string()),
	}
}

// endregion: --- Support
