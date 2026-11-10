use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process_group, test_kill_process_group};
use tokio::io::AsyncRead;
use tokio::process::Child;
use tokio::sync::{Notify, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep, timeout_at};

use cornercase_core::protocol::{BridgeMessage, LINE_TOO_LONG, USAGE_EXIT};

use super::lines::{self, End};

pub(super) const PATIENCE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(50);
const SIGNALLED: i32 = 128;
pub(super) const UNKNOWN: i32 = -1;

struct Readers {
    sender: mpsc::Sender<BridgeMessage>,
    reaped: watch::Receiver<Option<Instant>>,
    longest: usize,
    too_long: Arc<Notify>,
}

impl Readers {
    fn spawn<R>(&self, stream: R, message: fn(String) -> BridgeMessage) -> JoinHandle<End>
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let forward = lines::forward(stream, message, self.sender.clone(), self.reaped.clone(), self.longest);
        let too_long = Arc::clone(&self.too_long);
        tokio::spawn(async move {
            let end = forward.await;
            if end == End::TooLong {
                too_long.notify_one();
            }
            end
        })
    }
}

pub(super) fn supervise(
    mut child: Child,
    sender: mpsc::Sender<BridgeMessage>,
    stop: oneshot::Receiver<()>,
    longest: usize,
) {
    let (deadline, reaped) = watch::channel(None);
    let too_long = Arc::new(Notify::new());
    let readers =
        read(&mut child, &Readers { sender: sender.clone(), reaped, longest, too_long: Arc::clone(&too_long) });
    let group = leader(&child);
    tokio::spawn(async move {
        let (status, until) = end(&mut child, stop, &too_long).await;
        sweep(group, until).await;
        deadline.send_replace(Some(Instant::now() + POLL));
        let mut ends = Vec::new();
        for reader in readers {
            ends.push(reader.await.unwrap_or(End::Closed));
        }
        for message in last(status, &ends) {
            let _ = sender.send(message).await;
        }
    });
}

fn read(child: &mut Child, readers: &Readers) -> Vec<JoinHandle<End>> {
    let stdout = child.stdout.take().map(|stream| readers.spawn(stream, BridgeMessage::Stdout));
    let stderr = child.stderr.take().map(|stream| readers.spawn(stream, BridgeMessage::Stderr));
    [stdout, stderr].into_iter().flatten().collect()
}

pub(super) fn last(status: io::Result<ExitStatus>, ends: &[End]) -> Vec<BridgeMessage> {
    if ends.contains(&End::TooLong) {
        vec![BridgeMessage::Stderr(LINE_TOO_LONG.to_owned()), BridgeMessage::Exit(USAGE_EXIT)]
    } else {
        vec![BridgeMessage::Exit(exit_code(status))]
    }
}

async fn end(child: &mut Child, stop: oneshot::Receiver<()>, too_long: &Notify) -> (io::Result<ExitStatus>, Instant) {
    tokio::select! {
        status = child.wait() => (status, Instant::now() + PATIENCE),
        _ = stop => terminate(child).await,
        () = too_long.notified() => terminate(child).await,
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
