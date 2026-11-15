use std::io;
use std::pin::pin;
use std::time::Duration;

use futures_util::future::{self, Either};
use futures_util::stream::{self, BoxStream, SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};

use super::Socket;
use crate::error::Error;
use crate::protocol::{ClientMessage, LARGEST_MESSAGE, ServerMessage};
use crate::transport::{Closed, Exit, Out, OutSink, StderrTail, Stdin};

const CLOSING: Duration = Duration::from_millis(500);
const WRAPPING: usize = r#"{"stdin_base64":""}"#.len();
const PIECE: usize = (LARGEST_MESSAGE - WRAPPING) / 4 * 3;

type Outgoing = SplitSink<Socket, Message>;
type Incoming = SplitStream<Socket>;

pub(super) async fn run(
    socket: Socket,
    silence: Duration,
    argv: Vec<String>,
    stdin: BoxStream<'static, Stdin>,
    out: OutSink,
) -> Result<Exit, Error> {
    let (mut sink, mut stream) = socket.split();
    let ended = exchange(pump(&mut sink, argv, stdin), read(&mut stream, out, silence)).await;
    let _ = timeout(CLOSING, farewell(&mut sink, &mut stream)).await;
    ended
}

async fn exchange(
    pumping: impl Future<Output = Result<(), WebSocketError>>,
    reading: impl Future<Output = Result<Exit, Error>>,
) -> Result<Exit, Error> {
    let pumping = pin!(pumping);
    let reading = pin!(reading);
    match future::select(reading, pumping).await {
        Either::Left((ended, _)) => ended,
        Either::Right((_, reading)) => reading.await,
    }
}

async fn pump(sink: &mut Outgoing, argv: Vec<String>, stdin: BoxStream<'static, Stdin>) -> Result<(), WebSocketError> {
    let mut messages = stream::iter([ClientMessage::Argv(argv)])
        .chain(stdin.flat_map(|input| stream::iter(messages(input))))
        .chain(stream::iter([ClientMessage::Eof]))
        .map(|message| encode(&message));
    sink.send_all(&mut messages).await
}

fn messages(input: Stdin) -> Vec<ClientMessage> {
    match input {
        Stdin::Text(text) => vec![ClientMessage::Stdin(text)],
        Stdin::Bytes(bytes) => bytes.chunks(PIECE).map(|piece| ClientMessage::StdinBase64(piece.to_vec())).collect(),
    }
}

fn encode(message: &ClientMessage) -> Result<Message, WebSocketError> {
    let text = serde_json::to_string(message).map_err(|error| WebSocketError::Io(io::Error::other(error)))?;
    Ok(Message::text(text))
}

async fn read(stream: &mut Incoming, mut out: OutSink, silence: Duration) -> Result<Exit, Error> {
    let mut tail = StderrTail::default();
    loop {
        let message = timeout(silence, stream.next()).await.map_err(|_| Error::Dropped)?;
        let line = match heard(message)? {
            Some(ServerMessage::Exit(code)) => return Ok(Exit { code, stderr: tail.finish() }),
            Some(ServerMessage::Stdout(line)) => Out::Stdout(line),
            Some(ServerMessage::Stderr(line)) => {
                tail.push(&line);
                Out::Stderr(line)
            }
            None => continue,
        };
        out.send(line).await.map_err(|Closed| Error::OutputClosed)?;
    }
}

fn heard(message: Option<Result<Message, WebSocketError>>) -> Result<Option<ServerMessage>, Error> {
    match message {
        Some(Ok(Message::Text(text))) => parse(&text).map(Some),
        Some(Ok(Message::Binary(_))) => Err(Error::Protocol("a binary frame".to_owned())),
        Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => Ok(None),
        Some(Ok(Message::Close(_))) | None => Err(Error::Dropped),
        Some(Err(error)) => Err(read_error(&error)),
    }
}

fn parse(text: &str) -> Result<ServerMessage, Error> {
    serde_json::from_str(text)
        .map_err(|_| Error::Protocol("not one `stdout`, `stderr` or `exit` with a value of its type".to_owned()))
}

fn read_error(error: &WebSocketError) -> Error {
    match error {
        WebSocketError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => Error::Dropped,
        WebSocketError::Capacity(_) | WebSocketError::Protocol(_) | WebSocketError::Utf8(_) => {
            Error::Protocol(error.to_string())
        }
        _ => Error::Dropped,
    }
}

async fn farewell(sink: &mut Outgoing, stream: &mut Incoming) {
    let _ = sink.close().await;
    while let Some(Ok(_)) = stream.next().await {}
}

#[cfg(test)]
mod tests;
