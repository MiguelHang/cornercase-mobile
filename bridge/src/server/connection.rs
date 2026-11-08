use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{Sink, SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::{Instant, Interval, MissedTickBehavior, interval_at, timeout};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{Bytes, Error, Message, Utf8Bytes};

use cornercase_core::protocol::{BridgeMessage, Refusal, USAGE_EXIT, first_message};

use super::{Config, log};
use crate::allowlist;
use crate::process::{self, Running};

use hearing::{Ending, Hearing};

mod hearing;
mod input;

const CLOSING: Duration = Duration::from_secs(2);
const PLUGIN: &str = "plugin";

type Socket = WebSocketStream<TcpStream>;

pub(super) async fn serve(mut socket: Socket, peer: SocketAddr, config: &Config) {
    match open(&mut socket, peer, config).await {
        Ok(running) => run(socket, running, peer, config).await,
        Err(refusal) => {
            log(format_args!("refused {peer}'s command: {refusal}"));
            refuse(&mut socket, refusal).await;
            close(&mut socket).await;
        }
    }
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

async fn run(socket: Socket, mut running: Running, peer: SocketAddr, config: &Config) {
    let (mut sink, stream) = socket.split();
    let (ears, mut hearing) = hearing::listen(config.silence);
    let mut reader = tokio::spawn(input::read(stream, running.take_stdin(), peer, ears));
    let ending = forward(&mut sink, &mut running, &mut hearing, config.ping).await;
    let code = running.kill().await;
    if let Ending::Refused(refusal) = &ending {
        log(format_args!("refused {peer}'s message: {refusal}"));
    }
    log(format_args!("closed {peer}'s connection because {ending}, with exit code {code}"));
    let farewell = async {
        if let Ending::Refused(refusal) = ending {
            refuse(&mut sink, refusal).await;
        }
        say_goodbye(&mut sink).await.is_ok() && (&mut reader).await.is_ok()
    };
    if !timeout(CLOSING, farewell).await.unwrap_or(false) {
        reader.abort();
    }
}

async fn forward(sink: &mut impl Outgoing, running: &mut Running, hearing: &mut Hearing, ping: Duration) -> Ending {
    let mut ticks = interval_at(Instant::now() + ping, ping);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        if let Err(ending) = step(sink, running, hearing, &mut ticks).await {
            return ending;
        }
    }
}

async fn step(
    sink: &mut impl Outgoing,
    running: &mut Running,
    hearing: &mut Hearing,
    ticks: &mut Interval,
) -> Result<(), Ending> {
    let (message, last) = tokio::select! {
        message = running.next() => outgoing(message)?,
        _ = ticks.tick() => (Message::Ping(Bytes::new()), false),
        ending = hearing.ending() => return Err(ending),
    };
    tokio::select! {
        sent = sink.send(message) => sent.map_err(Ending::Broken)?,
        ending = hearing.ending() => return Err(ending),
    }
    if last { Err(Ending::Finished) } else { Ok(()) }
}

fn outgoing(message: Option<BridgeMessage>) -> Result<(Message, bool), Ending> {
    let message = message.ok_or(Ending::Finished)?;
    let last = matches!(message, BridgeMessage::Exit(_));
    Ok((encode(&message).map_err(Ending::Broken)?, last))
}

trait Outgoing: Sink<Message, Error = Error> + Unpin {}

impl<S: Sink<Message, Error = Error> + Unpin> Outgoing for S {}

async fn refuse(sink: &mut impl Outgoing, refusal: Refusal) {
    for message in [BridgeMessage::Stderr(refusal.message), BridgeMessage::Exit(USAGE_EXIT)] {
        if send(sink, &message).await.is_err() {
            return;
        }
    }
}

async fn send(sink: &mut impl Outgoing, message: &BridgeMessage) -> Result<(), Error> {
    sink.send(encode(message)?).await
}

fn encode(message: &BridgeMessage) -> Result<Message, Error> {
    let text = serde_json::to_string(message).map_err(|error| Error::Io(io::Error::other(error)))?;
    Ok(Message::text(text))
}

async fn say_goodbye(sink: &mut impl Outgoing) -> Result<(), Error> {
    let frame = CloseFrame { code: CloseCode::Normal, reason: "".into() };
    sink.send(Message::Close(Some(frame))).await
}

async fn close(socket: &mut Socket) {
    if say_goodbye(socket).await.is_err() {
        return;
    }
    let _ = timeout(CLOSING, async { while let Some(Ok(_)) = socket.next().await {} }).await;
}

#[cfg(test)]
mod tests;
