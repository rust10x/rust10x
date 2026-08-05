mod event_base;
mod mock_aijob;
mod mock_tui;

use mock_aijob::ai_queue;
use mock_tui::event as tui_event;
use tokio::try_join;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Starting event design demo");

	try_join!(tui_event::run_demo(), ai_queue::run_demo())?;

	println!("Event design demo completed");
	Ok(())
}
