use crate::event_base::{EventBaseResult, MpmcRx, MpmcTx, new_mpmc_bounded};
use tokio::time::{Duration, sleep};
use tokio::try_join;

type AiJobTx = MpmcTx<AiJob>;
type AiJobRx = MpmcRx<AiJob>;

#[derive(Debug)]
pub struct AiJob {
	pub id: u64,
	pub prompt: String,
}

#[derive(Clone)]
pub struct AiQueue {
	job_tx: AiJobTx,
	job_rx: AiJobRx,
}

impl AiQueue {
	pub fn new() -> EventBaseResult<Self> {
		let (job_tx, job_rx) = new_mpmc_bounded("mock-ai-jobs", 8)?;
		Ok(Self { job_tx, job_rx })
	}

	pub async fn queue_job(&self, job: AiJob) -> EventBaseResult<()> {
		self.job_tx.send(job).await
	}

	pub async fn get_job_todo(&self) -> EventBaseResult<AiJob> {
		self.job_rx.recv().await
	}
}

pub async fn run_demo() -> EventBaseResult<()> {
	let queue = AiQueue::new()?;
	try_join!(simulate_producer(queue.clone()), simulate_worker(queue))?;
	Ok(())
}

// region:    --- Support

async fn simulate_producer(queue: AiQueue) -> EventBaseResult<()> {
	for id in 1..=3 {
		let prompt = format!("demo prompt {id}");
		println!("[ai producer] queued job {id}");
		queue.queue_job(AiJob { id, prompt }).await?;
	}

	Ok(())
}

async fn simulate_worker(queue: AiQueue) -> EventBaseResult<()> {
	for _ in 0..3 {
		let job = queue.get_job_todo().await?;
		println!("[ai worker] processing job {}", job.id);
		sleep(Duration::from_millis(150)).await;
		println!("[ai worker] completed: {}", job.prompt);
	}

	Ok(())
}

// endregion: --- Support
