// region:    --- Modules

mod app;
mod ipc;

// endregion: --- Modules

use crate::app::{Client, Server};
use crate::ipc::socket::{self, ServerListener};
use std::path::PathBuf;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = core::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result<()> {
	let socket_path = socket_path();
	println!("main       - socket: {}", socket_path.display());

	// -- Start the service before any client connects.
	let service = ServerListener::bind(&socket_path, Server::default()).await?;
	let service_task = tokio::spawn(async move {
		if let Err(err) = service.run().await {
			eprintln!("service    - run error: {err}");
		}
	});

	let client_a = Client::connect("client_a", &socket_path).await?;
	let client_b = Client::connect("client_b", &socket_path).await?;

	// -- Both clients issue overlapping requests at the same time.
	let (res_a, res_b) = tokio::join!(run_client_a(&client_a), run_client_b(&client_b));
	res_a?;
	res_b?;

	let counter = client_a.counter_get().await?;
	println!("main       - final counter: {counter}");

	drop(client_a);
	drop(client_b);
	service_task.abort();
	socket::unlink_if_exists(&socket_path)?;

	Ok(())
}

// region:    --- Support

/// Short per-process socket path, path limits are ~104 bytes on macOS and ~108 on Linux.
fn socket_path() -> PathBuf {
	std::env::temp_dir().join(format!("ipc-sock-{}.sock", std::process::id()))
}

async fn run_client_a(client: &Client) -> Result<()> {
	let label = client.label();

	// -- Three in-flight requests on one connection, multiply answers last.
	let (sum, product, counter) = tokio::try_join!(
		//
		client.add(2, 40),
		client.multiply(6, 7),
		client.counter_incr(5)
	)?;
	println!("{label}   - add(2, 40) = {sum}");
	println!("{label}   - multiply(6, 7) = {product}");
	println!("{label}   - counter_incr(5) -> {counter}");

	let counter = client.counter_get().await?;
	println!("{label}   - counter_get() = {counter}");

	Ok(())
}

async fn run_client_b(client: &Client) -> Result<()> {
	let label = client.label();

	let (product, sum, counter) = tokio::try_join!(
		//
		client.multiply(9, 9),
		client.add(100, 1),
		client.counter_incr(3)
	)?;
	println!("{label}   - multiply(9, 9) = {product}");
	println!("{label}   - add(100, 1) = {sum}");
	println!("{label}   - counter_incr(3) -> {counter}");

	let counter = client.counter_get().await?;
	println!("{label}   - counter_get() = {counter}");

	Ok(())
}

// endregion: --- Support
