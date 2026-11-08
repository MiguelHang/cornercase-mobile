use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{Error, Message, Utf8Bytes};

use cornercase_core::protocol::{BridgeMessage, Refusal, USAGE_EXIT, first_message};

use super::{Config, log};
use crate::allowlist;
use crate::process::{self, Running};

const CLOSING: Duration = Duration::from_secs(2);
const PLUGIN: &str = "plugin";

type Socket = WebSocketStream<TcpStream>;

pub(super) async fn serve(mut socket: Socket, peer: SocketAddr, config: &Config) {
    match open(&mut socket, peer, config).await {
        Ok(running) => forward(&mut socket, running).await,
        Err(refusal) => {
            log(format_args!("refused {peer}'s command: {refusal}"));
            refuse(&mut socket, refusal).await;
        }
    }
    close(&mut socket).await;
}

async fn open(socket: &mut Socket, peer: SocketAddr, config: &Config) -> Result<Running, Refusal> {
    let argv = first_message(&hello(socket, config.hello).await?)?;
    allowlist::allowed(&argv)?;
    let running = process::spawn(&config.binary, &argv)
        .map_err(|error| Refusal::new(&format!("could not run cornercase: {error}")))?;
    log(format_args!("{peer} runs {}", command(&argv)));
    Ok(running)
}

async fn hello(socket: &mut Socket, within: Duration) -> Result<Utf8Bytes, Refusal> {
    timeout(within, first_text(socket))
        .await
        .unwrap_or_else(|_| Err(Refusal::new(&format!("no `argv` within {within:?}"))))
}

async fn first_text(socket: &mut Socket) -> Result<Utf8Bytes, Refusal> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => return Ok(text),
            Some(Ok(Message::Binary(_))) => return Err(Refusal::new("the first message must be text, not binary")),
            Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
            Some(Ok(Message::Close(_))) | None => return Err(Refusal::new("the connection closed before `argv`")),
            Some(Err(error)) => return Err(Refusal::new(&format!("could not read the first message: {error}"))),
        }
    }
}

fn command(argv: &[String]) -> String {
    match argv {
        [first, action, ..] if first == PLUGIN => format!("{first} {action}"),
        [first, ..] => first.clone(),
        [] => String::new(),
    }
}

async fn forward(socket: &mut Socket, mut running: Running) {
    while let Some(message) = running.next().await {
        if send(socket, &message).await.is_err() {
            running.kill().await;
            return;
        }
    }
}

async fn refuse(socket: &mut Socket, refusal: Refusal) {
    for message in [BridgeMessage::Stderr(refusal.message), BridgeMessage::Exit(USAGE_EXIT)] {
        if send(socket, &message).await.is_err() {
            return;
        }
    }
}

async fn send(socket: &mut Socket, message: &BridgeMessage) -> Result<(), Error> {
    let text = serde_json::to_string(message).map_err(|error| Error::Io(io::Error::other(error)))?;
    socket.send(Message::text(text)).await
}

async fn close(socket: &mut Socket) {
    let frame = CloseFrame { code: CloseCode::Normal, reason: "".into() };
    if socket.close(Some(frame)).await.is_err() {
        return;
    }
    let _ = timeout(CLOSING, async { while let Some(Ok(_)) = socket.next().await {} }).await;
}

#[cfg(test)]
mod tests;
