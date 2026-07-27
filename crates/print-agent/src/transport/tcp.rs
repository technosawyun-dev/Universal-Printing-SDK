//! Raw TCP:9100 ("JetDirect"/RAW) transport -- the most common way a network
//! thermal or kitchen printer accepts ESC/POS bytes directly, no driver
//! needed. Matches POS_system/Restaurant Platform's existing agents'
//! `/print-tcp` behavior (see docs/PROTOCOL.md's compatibility note).

use crate::error::AgentError;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn send(host: &str, port: u16, bytes: &[u8]) -> Result<(), AgentError> {
    let addr = format!("{host}:{port}");
    let mut stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr))
        .await
        .map_err(|_| AgentError::TransportError(format!("connect to {addr} timed out")))?
        .map_err(|e| AgentError::TransportError(format!("connect to {addr} failed: {e}")))?;
    stream.write_all(bytes).await.map_err(|e| AgentError::TransportError(format!("write to {addr} failed: {e}")))?;
    stream.flush().await.map_err(|e| AgentError::TransportError(format!("flush to {addr} failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn sends_bytes_to_a_listening_tcp_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 5];
            socket.read_exact(&mut buf).await.unwrap();
            buf
        });

        send(&addr.ip().to_string(), addr.port(), b"hello").await.unwrap();
        let received = server.await.unwrap();
        assert_eq!(received, b"hello");
    }

    #[tokio::test]
    async fn reports_a_transport_error_when_nothing_is_listening() {
        // Port 1 is reserved and nothing should be listening on it.
        let result = send("127.0.0.1", 1, b"x").await;
        assert!(result.is_err());
    }
}
