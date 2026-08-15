use derive_more::Display;

pub type AijobResult<T> = core::result::Result<T, AijobError>;

#[derive(Debug, Display)]
pub enum AijobError {
	#[display("AI job queue is unavailable")]
	QueueUnavailable,

	#[display("AI job queue is closed")]
	QueueClosed,
}

impl std::error::Error for AijobError {}

pub type AiJobError = AijobError;
pub type AiJobResult<T> = AijobResult<T>;
