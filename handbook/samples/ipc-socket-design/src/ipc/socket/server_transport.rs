//! Service side: accept loop, one task per connection, application logic behind `Handler`.
//!
//! Each connection reads requests sequentially but executes them in their own
//! task, so responses on a single connection can complete out of order. A writer
//! task owns the write half and serializes the outgoing frames.

use super::request_handler::RequestHandler;
use super::wire::{read_frame, write_frame};
use super::{Request, Response};
use crate::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

/// Bound service, ready to accept client connections.
pub struct ServerListener<H> {
	listener: UnixListener,
	handler: Arc<H>,
}

/// Removes a stale socket file, ignoring a missing path.
pub fn unlink_if_exists(socket_path: &Path) -> Result<()> {
	match std::fs::remove_file(socket_path) {
		Ok(()) => Ok(()),
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(err) => Err(err.into()),
	}
}

impl<H: RequestHandler> ServerListener<H> {
	/// Binds the listener, unlinking a stale socket path first.
	pub async fn bind(socket_path: impl AsRef<Path>, handler: H) -> Result<Self> {
		let socket_path = socket_path.as_ref();
		unlink_if_exists(socket_path)?;
		let listener = UnixListener::bind(socket_path)?;

		Ok(Self {
			listener,
			handler: Arc::new(handler),
		})
	}

	/// Runs the accept loop, spawning one task per connection.
	pub async fn run(self) -> Result<()> {
		let ServerListener {
			listener,
			handler: state,
		} = self;

		loop {
			let (stream, _addr) = listener.accept().await?;
			let state = state.clone();
			tokio::spawn(async move {
				if let Err(err) = handle_conn(stream, state).await {
					eprintln!("service    - connection error: {err}");
				}
			});
		}
	}
}

// region:    --- Support

async fn handle_conn<H: RequestHandler>(stream: UnixStream, handler: Arc<H>) -> Result<()> {
	let (mut reader, mut writer) = stream.into_split();
	let (res_tx, mut res_rx) = mpsc::channel::<Response<H::Reply>>(32);

	// -- Single writer task, so concurrent request tasks cannot interleave frames.
	let writer_task = tokio::spawn(async move {
		while let Some(response) = res_rx.recv().await {
			if let Err(err) = write_frame(&mut writer, &response).await {
				eprintln!("service    - write error: {err}");
				break;
			}
		}
	});

	// -- Read loop, one task per request.
	while let Some(request) = read_frame::<_, Request<H::Method>>(&mut reader).await? {
		let handler = handler.clone();
		let res_tx = res_tx.clone();
		tokio::spawn(async move {
			let response = execute(handler.as_ref(), request).await;
			let _ = res_tx.send(response).await;
		});
	}

	drop(res_tx);
	writer_task.await?;

	Ok(())
}

/// Runs one request through the handler, echoing the request id on the response.
async fn execute<H: RequestHandler>(handler: &H, request: Request<H::Method>) -> Response<H::Reply> {
	let Request { id, method } = request;
	println!("service    - exec  #{id} {method:?}");

	let reply = handler.exec(method).await;

	Response { id, reply }
}

// endregion: --- Support
