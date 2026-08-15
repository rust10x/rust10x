use super::aijob_error::{AiJobError, AiJobResult};
use crate::event_base::{EventBaseError, MpmcRx, MpmcTx, new_mpmc_bounded};
use tokio::time::{Duration, sleep};
use tokio::try_join;

#[derive(Debug)]
pub struct AiJob {
	pub id: u64,
	pub prompt: String,
}

pub fn new_ai_job_channel() -> AiJobResult<(AiJobTx, AiJobRx)> {
	let (tx, rx) = new_mpmc_bounded("mock-ai-jobs", 8).map_err(map_event_base_error)?;
	Ok((AiJobTx { inner: tx }, AiJobRx { inner: rx }))
}

// region:    --- AiJobTx

#[derive(Clone)]
pub struct AiJobTx {
	inner: MpmcTx<AiJob>,
}

impl AiJobTx {
	pub async fn exec_request(&self, job: AiJob) -> AiJobResult<()> {
		self.inner.send(job).await.map_err(map_event_base_error)
	}
}

// endregion: --- AiJobTx

// region:    --- AiJobRx

#[derive(Clone)]
pub struct AiJobRx {
	inner: MpmcRx<AiJob>,
}

impl AiJobRx {
	pub async fn next_request(&self) -> AiJobResult<AiJob> {
		self.inner.recv().await.map_err(map_event_base_error)
	}
}

// endregion: --- AiJobRx

pub async fn run_demo() -> crate::Result<()> {
	let (job_tx, job_rx) = new_ai_job_channel()?;
	try_join!(simulate_producer(job_tx), simulate_worker(job_rx))?;
	Ok(())
}

// region:    --- Support

async fn simulate_producer(job_tx: AiJobTx) -> crate::Result<()> {
	for id in 1..=3 {
		let prompt = format!("demo prompt {id}");
		println!("[ai producer] queued job {id}");
		job_tx.exec_request(AiJob { id, prompt }).await?;
	}

	Ok(())
}

async fn simulate_worker(job_rx: AiJobRx) -> crate::Result<()> {
	for _ in 0..3 {
		let job = job_rx.next_request().await?;
		println!("[ai worker] processing job {}", job.id);
		sleep(Duration::from_millis(150)).await;
		println!("[ai worker] completed: {}", job.prompt);
	}

	Ok(())
}

fn map_event_base_error(error: EventBaseError) -> AiJobError {
	// Keep the Level 1 to Level 3 classification visible at this boundary
	// instead of hiding it behind a general From implementation.
	if error.is_disconnected() {
		AiJobError::QueueClosed
	} else {
		AiJobError::QueueUnavailable
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_mock_aijob_map_event_base_error_disconnected() -> Result<()> {
		// -- Setup & Fixtures
		let tx_error = EventBaseError::TxDisconnected { name: "mock-ai-jobs" };
		let rx_error = EventBaseError::RxDisconnected { name: "mock-ai-jobs" };

		// -- Exec
		let tx_result = map_event_base_error(tx_error);
		let rx_result = map_event_base_error(rx_error);

		// -- Check
		assert!(matches!(tx_result, AiJobError::QueueClosed));
		assert!(matches!(rx_result, AiJobError::QueueClosed));

		Ok(())
	}

	#[test]
	fn test_mock_aijob_map_event_base_error_configuration() -> Result<()> {
		// -- Setup & Fixtures
		let error = EventBaseError::InvalidCapacity {
			name: "mock-ai-jobs",
			capacity: 0,
		};

		// -- Exec
		let result = map_event_base_error(error);

		// -- Check
		assert!(matches!(result, AiJobError::QueueUnavailable));

		Ok(())
	}
}

// endregion: --- Tests
