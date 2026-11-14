use std::time::Duration;

use futures_util::StreamExt;
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use serde::Deserialize;
use tokio::time::{self, Instant};
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::USER_AGENT;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};

use super::code::MachineCode;
use super::route::{Address, Token};
use crate::error::Error;
use crate::transport::websocket::{Socket, connect_error};

const LARGEST_REPLY: usize = 4096;
const ANSWERED: [Error; 3] = [Error::PairingFailed, Error::MalformedSecret, Error::PairingRefused];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairConfig {
    pub deadline: Duration,
}

impl Default for PairConfig {
    fn default() -> Self {
        Self { deadline: Duration::from_secs(10) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PairError {
    #[error("the machine code has no pairing code of cornercase")]
    NoCode,
    #[error(transparent)]
    Answered(Error),
    #[error(
        "no address of the Mac answered: is phone app on in cornercase, and the phone on its network or on Tailscale?"
    )]
    NoAnswer(Vec<(Address, Error)>),
}

#[derive(Deserialize)]
struct Reply {
    secret: String,
}

type Attempt = BoxFuture<'static, (usize, Result<Token, Error>)>;

pub async fn pair(code: &MachineCode, user_agent: &str, config: PairConfig) -> Result<Token, PairError> {
    let pairing = code.pairing_code().ok_or(PairError::NoCode)?;
    let addresses = code.addresses();
    let deadline = Instant::now() + config.deadline;
    let mut racing: FuturesUnordered<Attempt> = addresses
        .iter()
        .enumerate()
        .map(|(index, address)| attempt(index, request(address, pairing, user_agent)))
        .collect();
    let mut failures = vec![None; addresses.len()];
    while let Ok(Some((index, result))) = time::timeout_at(deadline, racing.next()).await {
        match result {
            Ok(secret) => return Ok(secret),
            Err(error) => failures[index] = Some(error),
        }
    }
    Err(verdict(addresses, failures))
}

fn request(address: &Address, code: &Token, user_agent: &str) -> Result<Request, Error> {
    let url = format!("{address}?code={}", code.as_str());
    let mut request = url.into_client_request().map_err(connect_error)?;
    let agent = HeaderValue::from_str(user_agent)
        .map_err(|_| Error::Connect("the app's name cannot go in a header".to_owned()))?;
    request.headers_mut().insert(USER_AGENT, agent);
    Ok(request)
}

fn attempt(index: usize, request: Result<Request, Error>) -> Attempt {
    Box::pin(async move {
        let result = match request {
            Ok(request) => exchange(request).await,
            Err(error) => Err(error),
        };
        (index, result)
    })
}

async fn exchange(request: Request) -> Result<Token, Error> {
    let (mut socket, _) = connect_async_with_config(request, Some(small()), true).await.map_err(refused)?;
    let secret = secret(next(&mut socket).await)?;
    ended(next(&mut socket).await)?;
    Ok(secret)
}

fn small() -> WebSocketConfig {
    WebSocketConfig::default().max_frame_size(Some(LARGEST_REPLY)).max_message_size(Some(LARGEST_REPLY))
}

fn refused(error: WebSocketError) -> Error {
    match connect_error(error) {
        Error::Rejected { status: 403 } => Error::PairingRefused,
        error => error,
    }
}

async fn next(socket: &mut Socket) -> Option<Result<Message, WebSocketError>> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            other => return other,
        }
    }
}

fn secret(message: Option<Result<Message, WebSocketError>>) -> Result<Token, Error> {
    match message {
        Some(Ok(Message::Text(text))) => checked(text.as_bytes()),
        Some(Ok(Message::Binary(bytes))) => checked(&bytes),
        Some(Ok(Message::Close(frame))) => Err(closed_early(frame)),
        Some(Ok(_)) => Err(Error::Protocol("the Mac sent a raw frame instead of the secret".to_owned())),
        Some(Err(_)) | None => Err(Error::Dropped),
    }
}

fn checked(reply: &[u8]) -> Result<Token, Error> {
    let Reply { secret } = serde_json::from_slice(reply).map_err(|_| Error::MalformedSecret)?;
    match secret.split_once('.') {
        Some((id, key)) if hex(id) && hex(key) => Ok(Token::new(secret)),
        _ => Err(Error::MalformedSecret),
    }
}

fn hex(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn closed_early(frame: Option<CloseFrame>) -> Error {
    match frame.map(|frame| frame.code) {
        Some(CloseCode::Policy) => Error::PairingRefused,
        Some(CloseCode::Error) => Error::PairingFailed,
        _ => Error::Protocol("the Mac closed the pairing without a secret".to_owned()),
    }
}

fn ended(message: Option<Result<Message, WebSocketError>>) -> Result<(), Error> {
    match message {
        Some(Ok(Message::Close(Some(frame)))) if frame.code == CloseCode::Normal => Ok(()),
        _ => Err(Error::Protocol("the Mac did not close the pairing after the secret".to_owned())),
    }
}

fn verdict(addresses: Vec<Address>, failures: Vec<Option<Error>>) -> PairError {
    if let Some(answer) = ANSWERED.into_iter().find(|answer| failures.contains(&Some(answer.clone()))) {
        return PairError::Answered(answer);
    }
    let failures = failures.into_iter().map(|failure| failure.unwrap_or(Error::Timeout));
    PairError::NoAnswer(addresses.into_iter().zip(failures).collect())
}

#[cfg(test)]
mod tests;
