use std::net::SocketAddr;

use futures_util::StreamExt;
use futures_util::stream::SplitStream;
use tokio_tungstenite::tungstenite::Message;

use cornercase_core::protocol::{Input, Refusal, next_message};

use super::Socket;
use super::hearing::{Ears, Ending};
use crate::process::Stdin;
use crate::server::log;

pub(super) async fn read(mut stream: SplitStream<Socket>, mut stdin: Option<Stdin>, peer: SocketAddr, ears: Ears) {
    let ending = listen(&mut stream, stdin.as_mut(), peer, &ears).await;
    let refused = matches!(ending, Ending::Refused(_));
    ears.end(ending);
    if refused {
        while let Some(Ok(_)) = stream.next().await {}
    }
}

async fn listen(
    stream: &mut SplitStream<Socket>,
    mut stdin: Option<&mut Stdin>,
    peer: SocketAddr,
    ears: &Ears,
) -> Ending {
    loop {
        let message = match stream.next().await {
            Some(Ok(message)) => message,
            Some(Err(error)) => return Ending::Broken(error),
            None => return Ending::Closed,
        };
        ears.heard();
        match input(message) {
            Ok(Some(input)) => {
                ears.busy();
                write(stdin.as_deref_mut(), input, peer).await;
                ears.heard();
            }
            Ok(None) => {}
            Err(refusal) => return Ending::Refused(refusal),
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
