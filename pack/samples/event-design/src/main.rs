// region:    --- Modules

mod error;
mod event_base;
mod mock_aijob;
mod mock_tui;

pub use error::{Error, Result};
use mock_aijob::aijob_queue;
use mock_tui::event as tui_event;
use tokio::try_join;

// endregion: --- Modules

#[tokio::main]
async fn main() -> Result<()> {
	println!("Starting event design demo");

	try_join!(tui_event::run_demo(), aijob_queue::run_demo())?;

	println!("Event design demo completed");
	Ok(())
}
