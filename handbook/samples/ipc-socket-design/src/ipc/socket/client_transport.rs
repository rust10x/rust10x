//! Client side: one connection multiplexing many in-flight requests.
//!
//! The write half is guarded by an async mutex, and a background reader task
//! resolves pending calls by matching the response id with the request id.

use super::envelope::{Request, RequestId, RequestIdGen, Response};
use super::wire::{WireReader, WireWriter};
use crate::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::net::UnixStream;
use tokio::net::unix::OwnedReadHalf;
use tokio::net::unix::OwnedWriteHalf;
use tokio::sync::{Mutex as AsyncMutex, oneshot};
use tokio::task::JoinHandle;

/// In-flight calls, keyed by the request id carried on the wire.
type Pending<R> = Arc<Mutex<HashMap<RequestId, oneshot::Sender<R>>>>;

/// Connected client, safe to call concurrently through a shared reference.
///
/// `M` is the application method payload, `R` the application reply payload.
pub struct ClientConnection<M, R> {
	label: String,
	id_gen: RequestIdGen,
	writer: AsyncMutex<WireWriter<OwnedWriteHalf, Request<M>>>,
	pending: Pending<R>,
	reader_task: JoinHandle<()>,
	_method: PhantomData<fn(M)>,
}

impl<M, R> ClientConnection<M, R>
where
	M: Serialize + Send + 'static,
	R: DeserializeOwned + Send + 'static,
{
	/// Connects to the service socket and starts the background reader task.
	pub async fn connect(label: impl Into<String>, socket_path: impl AsRef<Path>) -> Result<Self> {
		let label = label.into();
		let stream = UnixStream::connect(socket_path.as_ref()).await?;
		let (reader, writer) = stream.into_split();

		let pending: Pending<R> = Arc::new(Mutex::new(HashMap::new()));
		let reader = WireReader::<_, Response<R>>::new(reader);
		let writer = WireWriter::<_, Request<M>>::new(writer);
		let reader_task = spawn_reader(label.clone(), reader, pending.clone());

		Ok(Self {
			label,
			id_gen: RequestIdGen::default(),
			writer: AsyncMutex::new(writer),
			pending,
			reader_task,
			_method: PhantomData,
		})
	}

	/// Sends one request and waits for the response with the matching id.
	///
	/// Multiple calls can be in flight at the same time on the same connection.
	pub async fn invoke(&self, method: M) -> Result<R> {
		let id = self.id_gen.next_id();
		let (res_tx, res_rx) = oneshot::channel();

		{
			let mut pending = self.pending.lock().map_err(|_| "client - pending mutex poisoned")?;
			pending.insert(id, res_tx);
		}

		let request = Request { id, method };
		let write_res = {
			let mut writer = self.writer.lock().await;
			writer.write_frame(&request).await
		};

		if let Err(err) = write_res {
			self.remove_pending(id);
			return Err(err);
		}

		let reply = res_rx
			.await
			.map_err(|_| format!("{} - connection closed before response #{id}", self.label))?;

		Ok(reply)
	}

	fn remove_pending(&self, id: RequestId) {
		if let Ok(mut pending) = self.pending.lock() {
			pending.remove(&id);
		}
	}
}

/// Accessors
impl<M, R> ClientConnection<M, R> {
	pub fn label(&self) -> &str {
		&self.label
	}
}

impl<M, R> Drop for ClientConnection<M, R> {
	fn drop(&mut self) {
		self.reader_task.abort();
	}
}

// region:    --- Support

fn spawn_reader<R>(label: String, mut reader: WireReader<OwnedReadHalf, Response<R>>, pending: Pending<R>) -> JoinHandle<()>
where
	R: DeserializeOwned + Send + 'static,
{
	tokio::spawn(async move {
		loop {
			match reader.read_frame().await {
				Ok(Some(response)) => {
					let res_tx = pending
						.lock()
						.ok()
						.and_then(|mut pending| pending.remove(&response.id));

					match res_tx {
						Some(res_tx) => {
							let _ = res_tx.send(response.reply);
						}
						None => eprintln!("{label} - no pending request for id {}", response.id),
					}
				}
				Ok(None) => break,
				Err(err) => {
					eprintln!("{label} - read error: {err}");
					break;
				}
			}
		}
	})
}

// endregion: --- Support
