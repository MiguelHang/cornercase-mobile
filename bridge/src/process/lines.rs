use std::future::pending;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::{mpsc, watch};
use tokio::time::{Instant, sleep_until};

use cornercase_core::protocol::BridgeMessage;

const BUFFER: usize = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum End {
    Closed,
    TooLong,
}

pub(super) async fn forward<R>(
    stream: R,
    message: fn(String) -> BridgeMessage,
    sender: mpsc::Sender<BridgeMessage>,
    mut deadline: watch::Receiver<Option<Instant>>,
    longest: usize,
) -> End
where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::with_capacity(BUFFER, stream);
    loop {
        let line = tokio::select! {
            biased;
            line = next_line(&mut reader, longest) => line,
            () = gave_up(&mut deadline) => return End::Closed,
        };
        let line = match line {
            Ok(line) => line,
            Err(end) => return end,
        };
        if sender.send(message(text(line))).await.is_err() {
            return End::Closed;
        }
    }
}

async fn next_line<R>(reader: &mut BufReader<R>, longest: usize) -> Result<Vec<u8>, End>
where
    R: AsyncRead + Unpin,
{
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf().await.map_err(|_| End::Closed)?;
        if available.is_empty() {
            return if line.is_empty() { Err(End::Closed) } else { Ok(line) };
        }
        let newline = available.iter().position(|&byte| byte == b'\n');
        let taken = newline.unwrap_or(available.len());
        if line.len() + taken > longest {
            return Err(End::TooLong);
        }
        line.extend_from_slice(&available[..taken]);
        reader.consume(newline.map_or(taken, |at| at + 1));
        if newline.is_some() {
            return Ok(line);
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
