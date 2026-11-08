use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process_group};
use tokio::process::Child;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout, timeout_at};

use cornercase_core::protocol::BridgeMessage;

use super::lines;

const PATIENCE: Duration = Duration::from_secs(2);
const SIGNALLED: i32 = 128;
pub(super) const UNKNOWN: i32 = -1;

pub(super) fn supervise(mut child: Child, sender: mpsc::Sender<BridgeMessage>, stop: oneshot::Receiver<()>) {
    let readers: Vec<JoinHandle<()>> = [
        child.stdout.take().map(|stream| tokio::spawn(lines::forward(stream, BridgeMessage::Stdout, sender.clone()))),
        child.stderr.take().map(|stream| tokio::spawn(lines::forward(stream, BridgeMessage::Stderr, sender.clone()))),
    ]
    .into_iter()
    .flatten()
    .collect();
    tokio::spawn(async move {
        let code = exit_code(end(&mut child, stop).await);
        finish(readers).await;
        let _ = sender.send(BridgeMessage::Exit(code)).await;
    });
}

async fn end(child: &mut Child, stop: oneshot::Receiver<()>) -> io::Result<ExitStatus> {
    tokio::select! {
        status = child.wait() => status,
        _ = stop => terminate(child).await,
    }
}

async fn terminate(child: &mut Child) -> io::Result<ExitStatus> {
    signal(child, Signal::TERM);
    if let Ok(status) = timeout(PATIENCE, child.wait()).await {
        return status;
    }
    signal(child, Signal::KILL);
    child.wait().await
}

fn signal(child: &Child, which: Signal) {
    if let Some(group) = child.id().and_then(|id| i32::try_from(id).ok()).and_then(Pid::from_raw) {
        let _ = kill_process_group(group, which);
    }
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
