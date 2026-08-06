//! Length-delimited framing over any async byte stream.
//!
//! Frame layout: `[u32 length little-endian][postcard payload]`

use crate::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

/// Upper bound on a single payload, guards against a bogus length prefix.
pub const MAX_FRAME_LEN: u32 = 8 * 1024 * 1024;

// region:    --- Types

#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct WireReader<R, T> {
	framed: FramedRead<R, LengthDelimitedCodec>,
	marker: PhantomData<fn() -> T>,
}

pub(super) struct WireWriter<W, T> {
	framed: FramedWrite<W, LengthDelimitedCodec>,
	marker: PhantomData<fn() -> T>,
}

// endregion: --- Types

// region:    --- Support

fn new_codec() -> LengthDelimitedCodec {
	LengthDelimitedCodec::builder()
		.length_field_type::<u32>()
		.length_field_offset(0)
		.little_endian()
		.length_adjustment(0)
		.num_skip(4)
		.max_frame_length(MAX_FRAME_LEN as usize)
		.new_codec()
}

// endregion: --- Support

#[cfg_attr(not(test), allow(dead_code))]
impl<R, T> WireReader<R, T>
where
	R: AsyncRead + Unpin,
{
	pub(super) fn new(reader: R) -> Self {
		Self {
			framed: FramedRead::new(reader, new_codec()),
			marker: PhantomData,
		}
	}
}

#[cfg_attr(not(test), allow(dead_code))]
impl<R, T> WireReader<R, T>
where
	R: AsyncRead + Unpin,
	T: DeserializeOwned,
{
	pub(super) async fn read_frame(&mut self) -> Result<Option<T>> {
		match self.framed.next().await {
			Some(Ok(payload)) => Ok(Some(postcard::from_bytes(&payload)?)),
			Some(Err(err)) => Err(err.into()),
			None => Ok(None),
		}
	}
}

impl<W, T> WireWriter<W, T>
where
	W: AsyncWrite + Unpin,
{
	pub(super) fn new(writer: W) -> Self {
		Self {
			framed: FramedWrite::new(writer, new_codec()),
			marker: PhantomData,
		}
	}
}

impl<W, T> WireWriter<W, T>
where
	W: AsyncWrite + Unpin,
	T: Serialize,
{
	pub(super) async fn write_frame(&mut self, value: &T) -> Result<()> {
		let payload = postcard::to_stdvec(value)?;
		let len = u32::try_from(payload.len())?;
		if len > MAX_FRAME_LEN {
			return Err(format!("wire - frame length {len} exceeds max {MAX_FRAME_LEN}").into());
		}

		self.framed.send(payload.into()).await?;
		self.framed.flush().await?;

		Ok(())
	}
}

/// Serializes `value` with postcard and writes it as one length-delimited frame.
#[allow(dead_code)]
pub async fn write_frame<W, T>(writer: &mut W, value: &T) -> Result<()>
where
	W: AsyncWrite + Unpin,
	T: Serialize,
{
	let mut writer = WireWriter::<&mut W, T>::new(writer);
	writer.write_frame(value).await
}

/// Reads one length-delimited frame and decodes it with postcard.
///
/// Returns `Ok(None)` on a clean end of stream, that is, when the peer closed
/// the connection on a frame boundary.
#[allow(dead_code)]
pub async fn read_frame<R, T>(reader: &mut R) -> Result<Option<T>>
where
	R: AsyncRead + Unpin,
	T: DeserializeOwned,
{
	// -- Length prefix
	let mut len_buf = [0u8; 4];
	let bytes_read = reader.read(&mut len_buf[..1]).await?;
	if bytes_read == 0 {
		return Ok(None);
	}
	reader.read_exact(&mut len_buf[1..]).await?;

	let len = u32::from_le_bytes(len_buf);
	if len > MAX_FRAME_LEN {
		return Err(format!("wire - frame length {len} exceeds max {MAX_FRAME_LEN}").into());
	}

	// -- Payload
	let mut payload = vec![0u8; len as usize];
	reader.read_exact(&mut payload).await?;
	let value = postcard::from_bytes(&payload)?;

	Ok(Some(value))
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

	use super::*;
	use crate::ipc::socket::{Request, Response};
	use tokio::io::{AsyncReadExt, AsyncWriteExt};

	#[tokio::test]
	async fn test_ipc_socket_wire_read_clean_eof() -> Result<()> {
		// -- Setup & Fixtures
		let mut reader = WireReader::<_, u8>::new(tokio::io::empty());

		// -- Exec
		let actual = reader.read_frame().await?;

		// -- Check
		assert!(actual.is_none());

		Ok(())
	}

	#[tokio::test]
	async fn test_ipc_socket_wire_read_partial_length_prefix() -> Result<()> {
		// -- Setup & Fixtures
		let (mut writer, reader_io) = tokio::io::duplex(64);
		writer.write_all(&[0x01, 0x02]).await?;
		drop(writer);
		let mut reader = WireReader::<_, u8>::new(reader_io);

		// -- Exec & Check
		assert!(reader.read_frame().await.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_ipc_socket_wire_read_truncated_payload() -> Result<()> {
		// -- Setup & Fixtures
		let (mut writer, reader_io) = tokio::io::duplex(64);
		writer.write_all(&3u32.to_le_bytes()).await?;
		writer.write_all(&[0x01, 0x02]).await?;
		drop(writer);
		let mut reader = WireReader::<_, u8>::new(reader_io);

		// -- Exec & Check
		assert!(reader.read_frame().await.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_ipc_socket_wire_reject_oversized_frame() -> Result<()> {
		// -- Setup & Fixtures
		let (mut writer, reader_io) = tokio::io::duplex(64);
		let oversized_len = (MAX_FRAME_LEN + 1).to_le_bytes();
		writer.write_all(&oversized_len).await?;
		drop(writer);
		let mut reader = WireReader::<_, u8>::new(reader_io);

		// -- Exec & Check
		assert!(reader.read_frame().await.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_ipc_socket_wire_max_frame_boundary() -> Result<()> {
		// -- Setup & Fixtures
		let payload = vec![0u8; MAX_FRAME_LEN as usize - 4];
		let serialized = postcard::to_stdvec(&payload)?;
		assert_eq!(serialized.len(), MAX_FRAME_LEN as usize);

		let (writer_io, reader_io) = tokio::io::duplex(MAX_FRAME_LEN as usize + 4);
		let mut writer = WireWriter::<_, Vec<u8>>::new(writer_io);
		let mut reader = WireReader::<_, Vec<u8>>::new(reader_io);

		// -- Exec
		writer.write_frame(&payload).await?;
		let actual = reader.read_frame().await?.ok_or("missing maximum frame")?;

		// -- Check
		assert_eq!(actual.len(), payload.len());

		Ok(())
	}

	#[tokio::test]
	async fn test_ipc_socket_wire_multiple_buffered_frames() -> Result<()> {
		// -- Setup & Fixtures
		let (writer_io, reader_io) = tokio::io::duplex(128);
		let mut writer = WireWriter::<_, u32>::new(writer_io);
		writer.write_frame(&11).await?;
		writer.write_frame(&22).await?;
		drop(writer);
		let mut reader = WireReader::<_, u32>::new(reader_io);

		// -- Exec
		let first = reader.read_frame().await?.ok_or("missing first frame")?;
		let second = reader.read_frame().await?.ok_or("missing second frame")?;
		let end = reader.read_frame().await?;

		// -- Check
		assert_eq!(first, 11);
		assert_eq!(second, 22);
		assert!(end.is_none());

		Ok(())
	}

	#[tokio::test]
	async fn test_ipc_socket_wire_little_endian_prefix() -> Result<()> {
		// -- Setup & Fixtures
		let value = 42u8;
		let payload = postcard::to_stdvec(&value)?;
		let (writer_io, reader_io) = tokio::io::duplex(64);
		let mut writer = WireWriter::<_, u8>::new(writer_io);
		let mut reader = reader_io;

		// -- Exec
		writer.write_frame(&value).await?;
		let mut prefix = [0u8; 4];
		reader.read_exact(&mut prefix).await?;
		let mut actual_payload = vec![0u8; payload.len()];
		reader.read_exact(&mut actual_payload).await?;

		// -- Check
		assert_eq!(prefix, [1, 0, 0, 0]);
		assert_eq!(actual_payload, payload);

		Ok(())
	}

	#[tokio::test]
	async fn test_ipc_socket_wire_typed_request_response_round_trip() -> Result<()> {
		// -- Setup & Fixtures
		let request = Request {
			id: 7u64.into(),
			method: String::from("ping"),
		};
		let (request_writer_io, request_reader_io) = tokio::io::duplex(128);
		let mut request_writer = WireWriter::<_, Request<String>>::new(request_writer_io);
		let mut request_reader = WireReader::<_, Request<String>>::new(request_reader_io);

		let response = Response {
			id: 7u64.into(),
			reply: String::from("pong"),
		};
		let (response_writer_io, response_reader_io) = tokio::io::duplex(128);
		let mut response_writer = WireWriter::<_, Response<String>>::new(response_writer_io);
		let mut response_reader = WireReader::<_, Response<String>>::new(response_reader_io);

		// -- Exec
		request_writer.write_frame(&request).await?;
		let actual_request = request_reader.read_frame().await?.ok_or("missing request frame")?;
		response_writer.write_frame(&response).await?;
		let actual_response = response_reader.read_frame().await?.ok_or("missing response frame")?;

		// -- Check
		assert_eq!(actual_request.id, request.id);
		assert_eq!(actual_request.method, request.method);
		assert_eq!(actual_response.id, response.id);
		assert_eq!(actual_response.reply, response.reply);

		Ok(())
	}
}

// endregion: --- Tests
