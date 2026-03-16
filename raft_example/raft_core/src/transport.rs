use crate::error::Result;
use crate::rpc::RpcMessage;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Length-prefixed framing: [4-byte big-endian length][JSON payload]
pub async fn send_message(stream: &mut TcpStream, msg: &RpcMessage) -> Result<()> {
    let payload = serde_json::to_vec(msg)?;
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&payload).await?;
    stream.flush().await?;
    Ok(())
}

pub async fn recv_message(stream: &mut TcpStream) -> Result<RpcMessage> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(crate::error::RaftError::Internal(
            "message too large".into(),
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    let msg: RpcMessage = serde_json::from_slice(&buf)?;
    Ok(msg)
}

/// Send a single RPC message to a remote address and return the response.
pub async fn rpc_call(addr: SocketAddr, msg: &RpcMessage) -> Result<RpcMessage> {
    let mut stream = TcpStream::connect(addr).await?;
    send_message(&mut stream, msg).await?;
    let resp = recv_message(&mut stream).await?;
    Ok(resp)
}

/// Start listening on `addr` and forward every incoming RPC message into `tx`.
/// Each connection is handled in its own task; the response from the node is
/// sent back through a oneshot channel bundled with the incoming message.
pub async fn start_listener(
    addr: SocketAddr,
    tx: mpsc::Sender<(RpcMessage, oneshot::Sender<RpcMessage>)>,
) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", addr);
    loop {
        let (mut stream, peer) = listener.accept().await?;
        let tx = tx.clone();
        tokio::spawn(async move {
            match handle_connection(&mut stream, &tx).await {
                Ok(()) => {}
                Err(e) => {
                    debug!("connection from {} closed: {}", peer, e);
                }
            }
        });
    }
}

async fn handle_connection(
    stream: &mut TcpStream,
    tx: &mpsc::Sender<(RpcMessage, oneshot::Sender<RpcMessage>)>,
) -> Result<()> {
    // Handle a single request per connection (simple and safe).
    let msg = recv_message(stream).await?;
    let (resp_tx, resp_rx) = oneshot::channel();
    if tx.send((msg, resp_tx)).await.is_err() {
        warn!("node channel closed");
        return Ok(());
    }
    match resp_rx.await {
        Ok(resp) => send_message(stream, &resp).await?,
        Err(_) => warn!("response channel dropped"),
    }
    Ok(())
}

/// Oneshot channel re-export so callers don't need to depend on tokio directly.
pub mod oneshot {
    pub use tokio::sync::oneshot::{channel, Receiver, Sender};
}
