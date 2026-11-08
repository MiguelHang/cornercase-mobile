use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::time::Duration;

use tokio::process::Child;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout_at};

use cornercase_core::protocol::BridgeMessage;

use super::lines;

const PATIENCE: Duration = Duration::from_secs(2);
const SIGNALLED: i32 = 128;
const UNKNOWN: i32 = -1;

pub(super) fn supervise(mut child: Child, sender: mpsc::Sender<BridgeMessage>) {
    let readers: Vec<JoinHandle<()>> = [
        child.stdout.take().map(|stream| tokio::spawn(lines::forward(stream, BridgeMessage::Stdout, sender.clone()))),
        child.stderr.take().map(|stream| tokio::spawn(lines::forward(stream, BridgeMessage::Stderr, sender.clone()))),
    ]
    .into_iter()
    .flatten()
    .collect();
    tokio::spawn(async move {
        let code = exit_code(child.wait().await);
        finish(readers).await;
        let _ = sender.send(BridgeMessage::Exit(code)).await;
    });
}

async fn finish(readers: Vec<JoinHandle<()>>) {
    let deadline = Instant::now() + PATIENCE;
    for mut reader in readers {
        if timeout_at(deadline, &mut reader).await.is_err() {
            reader.abort();
            let _ = reader.await;
        }
    }
}

pub(super) fn exit_code(status: io::Result<ExitStatus>) -> i32 {
    status
        .ok()
        .and_then(|status| status.code().or_else(|| status.signal().map(|signal| SIGNALLED + signal)))
        .unwrap_or(UNKNOWN)
}
