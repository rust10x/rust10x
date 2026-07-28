//! Length-delimited framing over any async byte stream.
//!
//! Frame layout: `[u32 length little-endian][postcard payload]`

use crate::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Upper bound on a single payload, guards against a bogus length prefix.
pub const MAX_FRAME_LEN: u32 = 8 * 1024 * 1024;

/// Serializes `value` with postcard and writes it as one length-delimited frame.
pub async fn write_frame<W, T>(writer: &mut W, value: &T) -> Result<()>
where
	W: AsyncWrite + Unpin,
	T: Serialize,
{
	let payload = postcard::to_stdvec(value)?;
	let len = u32::try_from(payload.len())?;
	if len > MAX_FRAME_LEN {
		return Err(format!("wire - frame length {len} exceeds max {MAX_FRAME_LEN}").into());
	}

	writer.write_all(&len.to_le_bytes()).await?;
	writer.write_all(&payload).await?;
	writer.flush().await?;

	Ok(())
}

/// Reads one length-delimited frame and decodes it with postcard.
///
/// Returns `Ok(None)` on a clean end of stream, that is, when the peer closed
/// the connection on a frame boundary.
pub async fn read_frame<R, T>(reader: &mut R) -> Result<Option<T>>
where
	R: AsyncRead + Unpin,
	T: DeserializeOwned,
{
	// -- Length prefix
	let mut len_buf = [0u8; 4];
	if let Err(err) = reader.read_exact(&mut len_buf).await {
		if err.kind() == std::io::ErrorKind::UnexpectedEof {
			return Ok(None);
		}
		return Err(err.into());
	}

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
