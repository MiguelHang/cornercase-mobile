use std::future::pending;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::{mpsc, watch};
use tokio::time::{Instant, sleep_until};

use cornercase_core::protocol::BridgeMessage;

const BUFFER: usize = 1 << 20;

pub(super) async fn forward<R>(
    stream: R,
    message: fn(String) -> BridgeMessage,
    sender: mpsc::Sender<BridgeMessage>,
    mut deadline: watch::Receiver<Option<Instant>>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::with_capacity(BUFFER, stream);
    loop {
        let mut line = Vec::new();
        let read = tokio::select! {
            biased;
            read = reader.read_until(b'\n', &mut line) => read,
            () = gave_up(&mut deadline) => return,
        };
        if matches!(read, Ok(0) | Err(_)) {
            return;
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if sender.send(message(text(line))).await.is_err() {
            return;
        }
    }
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

async fn gave_up(deadline: &mut watch::Receiver<Option<Instant>>) {
    let set = deadline.wait_for(Option::is_some).await.ok().and_then(|deadline| *deadline);
    match set {
        Some(deadline) => sleep_until(deadline).await,
        None => pending().await,
    }
}

#[cfg(test)]
mod tests;
