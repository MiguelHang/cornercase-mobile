use std::net::SocketAddr;

use futures_util::StreamExt;
use futures_util::stream::SplitStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use cornercase_core::protocol::{Input, Refusal, next_message};

use super::Socket;
use crate::process::Stdin;
use crate::server::log;

pub(super) async fn read(
    mut stream: SplitStream<Socket>,
    mut stdin: Option<Stdin>,
    peer: SocketAddr,
    refusals: mpsc::Sender<Refusal>,
) {
    while let Some(Ok(message)) = stream.next().await {
        match input(message) {
            Ok(Some(input)) => write(stdin.as_mut(), input, peer).await,
            Ok(None) => {}
            Err(refusal) => {
                let _ = refusals.send(refusal).await;
                while let Some(Ok(_)) = stream.next().await {}
                return;
            }
        }
    }
}

fn input(message: Message) -> Result<Option<Input>, Refusal> {
    match message {
        Message::Text(text) => next_message(&text).map(Some),
        Message::Binary(_) => Err(Refusal::new("a message must be text, not binary")),
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) | Message::Close(_) => Ok(None),
    }
}

async fn write(stdin: Option<&mut Stdin>, input: Input, peer: SocketAddr) {
    let Some(stdin) = stdin else {
        log(format_args!("dropped {peer}'s input: the process has no input"));
        return;
    };
    if let Err(error) = stdin.write(input).await {
        log(format_args!("dropped {peer}'s input: {error}"));
    }
}

#[cfg(test)]
mod tests;
