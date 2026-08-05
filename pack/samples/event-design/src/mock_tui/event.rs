use crate::event_base::{EventBaseResult, MpscRx, MpscTx, new_mpsc_bounded};
use tokio::time::{Duration, sleep};
use tokio::try_join;

pub type TuiTx = MpscTx<TuiEvent>;
pub type TuiRx = MpscRx<TuiEvent>;

#[derive(Debug)]
pub enum TuiEvent {
	Input(String),
	Resize { width: u16, height: u16 },
	Quit,
}

pub fn new_tui_channel() -> EventBaseResult<(TuiTx, TuiRx)> {
	new_mpsc_bounded("mock-tui", 16)
}

pub async fn run_demo() -> EventBaseResult<()> {
	let (tx, rx) = new_tui_channel()?;
	try_join!(simulate_input(tx), run_event_loop(rx))?;
	Ok(())
}

// region:    --- Support

async fn simulate_input(tx: TuiTx) -> EventBaseResult<()> {
	tx.send(TuiEvent::Input(String::from("open settings"))).await?;
	sleep(Duration::from_millis(100)).await;

	tx.send(TuiEvent::Resize { width: 120, height: 40 }).await?;
	sleep(Duration::from_millis(100)).await;

	tx.send(TuiEvent::Quit).await
}

async fn run_event_loop(mut rx: TuiRx) -> EventBaseResult<()> {
	loop {
		match rx.recv().await? {
			TuiEvent::Input(value) => println!("[tui] input: {value}"),
			TuiEvent::Resize { width, height } => {
				println!("[tui] resized to {width}x{height}");
			}
			TuiEvent::Quit => {
				println!("[tui] quit");
				return Ok(());
			}
		}
	}
}

// endregion: --- Support
