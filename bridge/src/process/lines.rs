use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

use cornercase_core::protocol::BridgeMessage;

const BUFFER: usize = 1 << 20;

pub(super) async fn forward<R>(stream: R, message: fn(String) -> BridgeMessage, sender: mpsc::Sender<BridgeMessage>)
where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::with_capacity(BUFFER, stream);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if sender.send(message(String::from_utf8_lossy(&line).into_owned())).await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
