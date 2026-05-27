use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

const MAX_FRAME_SIZE: usize = 64 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WireMessage {
    Handshake {
        public_key: String,
        username: String,
        version: u8,
        session_salt: String,
    },
    Text {
        id: Uuid,
        from: String,
        content: String,
        timestamp: i64,
    },
    Ping {
        timestamp: i64,
    },
    Pong {
        timestamp: i64,
    },
    Ack {
        message_id: Uuid,
    },
        
}

pub async fn write_frame<W>(writer: &mut W, payload: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    if payload.len() > MAX_FRAME_SIZE {
        bail!("frame too large");
    }

    let len = payload.len() as u32;
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;

    Ok(())
}

pub async fn read_frame<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;

    let len = u32::from_le_bytes(len_buf) as usize;

    if len > MAX_FRAME_SIZE {
        bail!("frame too large");
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;

    Ok(payload)
}

pub async fn write_message<W>(writer: &mut W, message: &WireMessage) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let payload = serde_json::to_vec(message)?;
    write_frame(writer, &payload).await
}

pub async fn read_message<R>(reader: &mut R) -> Result<WireMessage>
where
    R: AsyncRead + Unpin,
{
    let payload = read_frame(reader).await?;
    let message = serde_json::from_slice::<WireMessage>(&payload)?;
    Ok(message)
}