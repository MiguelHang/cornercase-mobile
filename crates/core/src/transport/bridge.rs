use std::fmt;
use std::io;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async_with_config};

use crate::error::Error;
use crate::protocol::LONGEST_LINE;

const LARGEST_ANSWER: usize = 7 * LONGEST_LINE;

#[cfg_attr(not(test), expect(dead_code, reason = "the bridge transport runs its commands over it"))]
pub(crate) type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeConfig {
    pub connect_timeout: Duration,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self { connect_timeout: Duration::from_secs(3) }
    }
}

pub struct BridgeTransport {
    url: String,
    config: BridgeConfig,
}

impl BridgeTransport {
    pub fn new(url: String, config: BridgeConfig) -> Self {
        Self { url, config }
    }

    #[cfg_attr(not(test), expect(dead_code, reason = "the bridge transport runs its commands over it"))]
    pub(crate) async fn connect(&self) -> Result<Socket, Error> {
        let upgrade = connect_async_with_config(self.url.as_str(), Some(websocket()), false);
        match tokio::time::timeout(self.config.connect_timeout, upgrade).await {
            Ok(Ok((socket, _))) => Ok(socket),
            Ok(Err(error)) => Err(connect_error(error)),
            Err(_) => Err(Error::Timeout),
        }
    }
}

impl fmt::Debug for BridgeTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("BridgeTransport").field("config", &self.config).finish_non_exhaustive()
    }
}

fn websocket() -> WebSocketConfig {
    WebSocketConfig::default().max_frame_size(Some(LARGEST_ANSWER)).max_message_size(Some(LARGEST_ANSWER))
}

fn connect_error(error: WebSocketError) -> Error {
    match error {
        WebSocketError::Http(response) if response.status() == StatusCode::UNAUTHORIZED => Error::Unauthorized,
        WebSocketError::Http(response) => Error::Rejected { status: response.status().as_u16() },
        WebSocketError::Io(error) => Error::Connect(network_error(&error)),
        WebSocketError::Url(_) | WebSocketError::HttpFormat(_) => Error::Connect("not a ws:// address".to_owned()),
        WebSocketError::Protocol(error) => Error::Connect(format!("the WebSocket upgrade failed: {error}")),
        _ => Error::Connect("the WebSocket upgrade failed".to_owned()),
    }
}

fn network_error(error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(_) => error.to_string(),
        None => error.kind().to_string(),
    }
}

#[cfg(test)]
mod tests;
