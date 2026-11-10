use std::io;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use cornercase_core::protocol::{BridgeMessage, LONGEST_LINE};

mod group;
mod input;
mod lines;

pub use input::Stdin;

const REMOVED: [&str; 3] = ["CORNERCASE_PANE", "CORNERCASE_SERVER", "CORNERCASE_REMOTE"];
const MESSAGES: usize = 16;

pub struct Running {
    messages: mpsc::Receiver<BridgeMessage>,
    stdin: Option<Stdin>,
    order: Option<oneshot::Sender<()>>,
    code: Option<i32>,
}

pub fn spawn(binary: &Path, argv: &[String]) -> io::Result<Running> {
    launch(binary, argv, LONGEST_LINE)
}

fn launch(binary: &Path, argv: &[String], longest: usize) -> io::Result<Running> {
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
    let mut child = command.spawn()?;
    let stdin = Stdin::new(child.stdin.take());
    let (sender, messages) = mpsc::channel(MESSAGES);
    let (order, stop) = oneshot::channel();
    group::supervise(child, sender, stop, longest);
    Ok(Running { messages, stdin: Some(stdin), order: Some(order), code: None })
}

impl Running {
    pub async fn next(&mut self) -> Option<BridgeMessage> {
        let message = self.messages.recv().await;
        if let Some(BridgeMessage::Exit(code)) = message {
            self.code = Some(code);
        }
        message
    }

    pub fn take_stdin(&mut self) -> Option<Stdin> {
        self.stdin.take()
    }

    pub async fn kill(mut self) -> i32 {
        drop(self.order.take());
        while self.next().await.is_some() {}
        self.code.unwrap_or(group::UNKNOWN)
    }
}

#[cfg(test)]
mod tests;
