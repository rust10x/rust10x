use crate::event_base::{EventBaseResult, MpmcRx, MpmcTx, new_mpmc_bounded};
use tokio::time::{Duration, sleep};
use tokio::try_join;

#[derive(Debug)]
pub struct AiJob {
	pub id: u64,
	pub prompt: String,
}

#[derive(Clone)]
pub struct AiJobTx {
	inner: MpmcTx<AiJob>,
}

impl AiJobTx {
	pub async fn exec_request(&self, job: AiJob) -> EventBaseResult<()> {
		self.inner.send(job).await
	}
}

#[derive(Clone)]
pub struct AiJobRx {
	inner: MpmcRx<AiJob>,
}

impl AiJobRx {
	pub async fn next_request(&self) -> EventBaseResult<AiJob> {
		self.inner.recv().await
	}
}

pub fn new_ai_job_channel() -> EventBaseResult<(AiJobTx, AiJobRx)> {
	let (tx, rx) = new_mpmc_bounded("mock-ai-jobs", 8)?;
	Ok((AiJobTx { inner: tx }, AiJobRx { inner: rx }))
}

pub async fn run_demo() -> EventBaseResult<()> {
	let (job_tx, job_rx) = new_ai_job_channel()?;
	try_join!(simulate_producer(job_tx), simulate_worker(job_rx))?;
	Ok(())
}

// region:    --- Support

async fn simulate_producer(job_tx: AiJobTx) -> EventBaseResult<()> {
	for id in 1..=3 {
		let prompt = format!("demo prompt {id}");
		println!("[ai producer] queued job {id}");
		job_tx.exec_request(AiJob { id, prompt }).await?;
	}

	Ok(())
}

async fn simulate_worker(job_rx: AiJobRx) -> EventBaseResult<()> {
	for _ in 0..3 {
		let job = job_rx.next_request().await?;
		println!("[ai worker] processing job {}", job.id);
		sleep(Duration::from_millis(150)).await;
		println!("[ai worker] completed: {}", job.prompt);
	}

	Ok(())
}

// endregion: --- Support
