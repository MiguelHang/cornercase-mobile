use std::fmt;
use std::io;
use std::time::Duration;

use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async_with_config};

use super::{Exit, OutSink, Stdin, Transport};
use crate::error::Error;
use crate::protocol::LONGEST_LINE;

mod session;

const LARGEST_ANSWER: usize = 7 * LONGEST_LINE;

pub(crate) type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketConfig {
    pub connect_timeout: Duration,
    pub silence_timeout: Duration,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self { connect_timeout: Duration::from_secs(3), silence_timeout: Duration::from_secs(45) }
    }
}

pub struct WebSocketTransport {
    url: String,
    secret: String,
    config: SocketConfig,
}

impl WebSocketTransport {
    pub fn new(url: String, secret: String, config: SocketConfig) -> Self {
        Self { url, secret, config }
    }

    pub(crate) async fn connect(&self) -> Result<Socket, Error> {
        let request = self.request()?;
        let upgrade = connect_async_with_config(request, Some(websocket()), true);
        match tokio::time::timeout(self.config.connect_timeout, upgrade).await {
            Ok(Ok((socket, _))) => Ok(socket),
            Ok(Err(error)) => Err(refused(error)),
            Err(_) => Err(Error::Timeout),
        }
    }

    fn request(&self) -> Result<Request, Error> {
        let mut request = self.url.as_str().into_client_request().map_err(connect_error)?;
        let mut value = HeaderValue::from_str(&format!("Bearer {}", self.secret)).map_err(|_| unfit_secret())?;
        value.set_sensitive(true);
        request.headers_mut().insert(AUTHORIZATION, value);
        Ok(request)
    }
}

impl Transport for WebSocketTransport {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>> {
        async move { session::run(self.connect().await?, self.config.silence_timeout, argv, stdin, out).await }.boxed()
    }
}

impl fmt::Debug for WebSocketTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("WebSocketTransport").field("config", &self.config).finish_non_exhaustive()
    }
}

fn websocket() -> WebSocketConfig {
    WebSocketConfig::default().max_frame_size(Some(LARGEST_ANSWER)).max_message_size(Some(LARGEST_ANSWER))
}

fn refused(error: WebSocketError) -> Error {
    match connect_error(error) {
        Error::Rejected { status: 403 } => Error::NotPaired,
        error => error,
    }
}

fn unfit_secret() -> Error {
    Error::Secrets("the saved secret cannot go in a header".to_owned())
}

pub(crate) fn connect_error(error: WebSocketError) -> Error {
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
        None => "the address could not be resolved or reached".to_owned(),
    }
}

#[cfg(test)]
mod tests;
