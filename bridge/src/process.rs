use std::io;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;
use tokio::sync::mpsc;

use cornercase_core::protocol::BridgeMessage;

mod group;
mod lines;

const REMOVED: [&str; 3] = ["CORNERCASE_PANE", "CORNERCASE_SERVER", "CORNERCASE_REMOTE"];
const MESSAGES: usize = 16;

pub struct Running {
    messages: mpsc::Receiver<BridgeMessage>,
}

pub fn spawn(binary: &Path, argv: &[String]) -> io::Result<Running> {
    let mut command = Command::new(binary);
    command
        .args(argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true);
    for variable in REMOVED {
        command.env_remove(variable);
    }
    let child = command.spawn()?;
    let (sender, messages) = mpsc::channel(MESSAGES);
    group::supervise(child, sender);
    Ok(Running { messages })
}

impl Running {
    pub async fn next(&mut self) -> Option<BridgeMessage> {
        self.messages.recv().await
    }
}

#[cfg(test)]
mod tests;
