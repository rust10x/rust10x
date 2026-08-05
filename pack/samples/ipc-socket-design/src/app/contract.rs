//! Application payloads carried inside the transport envelope.

use serde::{Deserialize, Serialize};

// region:    --- Types

/// Operations exposed by the service.
///
/// The enum shape gives a single decode step and an exhaustive match, so adding
/// an operation is compiler guided.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Call {
	Add(BiParams),
	Multiply(BiParams),
	CounterIncr(CounterIncrParams),
	CounterGet,
}

/// Two operand parameters, shared by `Add` and `Multiply`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BiParams {
	pub a: i64,
	pub b: i64,
}

/// Parameters for incrementing the shared counter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CounterIncrParams {
	pub by: i64,
}

/// Outcome of one call, carried back to the caller as data rather than a transport error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallResult {
	Value(i64),
	Error(String),
}

// endregion: --- Types
