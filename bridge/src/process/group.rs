use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process_group, test_kill_process_group};
use tokio::process::Child;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep, timeout_at};

use cornercase_core::protocol::BridgeMessage;

use super::lines;

pub(super) const PATIENCE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(50);
const SIGNALLED: i32 = 128;
pub(super) const UNKNOWN: i32 = -1;

pub(super) fn supervise(mut child: Child, sender: mpsc::Sender<BridgeMessage>, stop: oneshot::Receiver<()>) {
    let (deadline, gone) = watch::channel(None);
    let readers = read(&mut child, &sender, &gone);
    let group = leader(&child);
    tokio::spawn(async move {
        let (status, until) = end(&mut child, stop).await;
        sweep(group, until).await;
        deadline.send_replace(Some(Instant::now() + POLL));
        for reader in readers {
            let _ = reader.await;
        }
        let _ = sender.send(BridgeMessage::Exit(exit_code(status))).await;
    });
}

fn read(
    child: &mut Child,
    sender: &mpsc::Sender<BridgeMessage>,
    reaped: &watch::Receiver<Option<Instant>>,
) -> Vec<JoinHandle<()>> {
    let stdout = child
        .stdout
        .take()
        .map(|stream| tokio::spawn(lines::forward(stream, BridgeMessage::Stdout, sender.clone(), reaped.clone())));
    let stderr = child
        .stderr
        .take()
        .map(|stream| tokio::spawn(lines::forward(stream, BridgeMessage::Stderr, sender.clone(), reaped.clone())));
    [stdout, stderr].into_iter().flatten().collect()
}

async fn end(child: &mut Child, stop: oneshot::Receiver<()>) -> (io::Result<ExitStatus>, Instant) {
    tokio::select! {
        status = child.wait() => (status, Instant::now() + PATIENCE),
        _ = stop => terminate(child).await,
    }
}

async fn terminate(child: &mut Child) -> (io::Result<ExitStatus>, Instant) {
    let deadline = Instant::now() + PATIENCE;
    signal(child, Signal::TERM);
    if let Ok(status) = timeout_at(deadline, child.wait()).await {
        return (status, deadline);
    }
    signal(child, Signal::KILL);
    (child.wait().await, deadline)
}

fn leader(child: &Child) -> Option<Pid> {
    child.id().and_then(|id| i32::try_from(id).ok()).and_then(Pid::from_raw)
}

fn signal(child: &Child, which: Signal) {
    if let Some(group) = leader(child) {
        let _ = kill_process_group(group, which);
    }
}

async fn sweep(group: Option<Pid>, deadline: Instant) {
    let Some(group) = group else { return };
    if !emptied(group, deadline).await {
        let _ = kill_process_group(group, Signal::KILL);
        emptied(group, deadline + PATIENCE).await;
    }
}

async fn emptied(group: Pid, deadline: Instant) -> bool {
    while test_kill_process_group(group).is_ok() {
        if Instant::now() >= deadline {
            return false;
        }
        sleep(POLL).await;
    }
    true
}

pub(super) fn exit_code(status: io::Result<ExitStatus>) -> i32 {
    status
        .ok()
        .and_then(|status| status.code().or_else(|| status.signal().map(|signal| SIGNALLED + signal)))
        .unwrap_or(UNKNOWN)
}
