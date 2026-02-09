use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
use tracing::{debug, trace};

const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024; // 16 MB

/// Send a length-prefixed bincode message over an async writer.
pub async fn send_message<W, T>(writer: &mut W, msg: &T) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
    T: Serialize,
{
    let data = bincode::serialize(msg)?;
    let len = data.len() as u32;
    trace!("Sending message: {} bytes", len);
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&data).await?;
    writer.flush().await?;
    Ok(())
}

/// Receive a length-prefixed bincode message from an async reader.
pub async fn recv_message<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncReadExt + Unpin,
    T: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf);

    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!("Message too large: {} bytes (max {})", len, MAX_MESSAGE_SIZE);
    }

    trace!("Receiving message: {} bytes", len);
    let mut data = vec![0u8; len as usize];
    reader.read_exact(&mut data).await?;
    let msg = bincode::deserialize(&data)?;
    Ok(msg)
}

/// Create a named pipe server instance.
pub fn create_server(pipe_name: &str) -> Result<NamedPipeServer> {
    debug!("Creating named pipe server: {}", pipe_name);
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(pipe_name)?;
    Ok(server)
}

/// Create a subsequent server instance (for accepting multiple clients).
pub fn create_server_instance(pipe_name: &str) -> Result<NamedPipeServer> {
    let server = ServerOptions::new()
        .first_pipe_instance(false)
        .create(pipe_name)?;
    Ok(server)
}

/// Connect to a named pipe server as a client.
pub async fn connect_client(pipe_name: &str) -> Result<NamedPipeClient> {
    debug!("Connecting to named pipe: {}", pipe_name);
    // Retry a few times since the server may not be ready yet.
    let mut attempts = 0;
    loop {
        match ClientOptions::new().open(pipe_name) {
            Ok(client) => return Ok(client),
            Err(e) if attempts < 10 => {
                attempts += 1;
                debug!(
                    "Pipe not ready (attempt {}), retrying: {}",
                    attempts, e
                );
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
}
