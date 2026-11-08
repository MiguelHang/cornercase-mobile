use std::io;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use cornercase_core::protocol::{BridgeMessage, Input};

mod group;
mod input;
mod lines;

const REMOVED: [&str; 3] = ["CORNERCASE_PANE", "CORNERCASE_SERVER", "CORNERCASE_REMOTE"];
const MESSAGES: usize = 16;

pub struct Running {
    messages: mpsc::Receiver<BridgeMessage>,
    input: input::Writer,
    order: Option<oneshot::Sender<()>>,
    code: Option<i32>,
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
    let mut child = command.spawn()?;
    let input = input::Writer::new(child.stdin.take());
    let (sender, messages) = mpsc::channel(MESSAGES);
    let (order, stop) = oneshot::channel();
    group::supervise(child, sender, stop);
    Ok(Running { messages, input, order: Some(order), code: None })
}

impl Running {
    pub async fn next(&mut self) -> Option<BridgeMessage> {
        let message = self.messages.recv().await;
        if let Some(BridgeMessage::Exit(code)) = message {
            self.code = Some(code);
        }
        message
    }

    pub async fn write(&mut self, input: Input) -> io::Result<()> {
        self.input.write(input).await
    }

    pub async fn kill(mut self) -> i32 {
        drop(self.order.take());
        while self.next().await.is_some() {}
        self.code.unwrap_or(group::UNKNOWN)
    }
}

#[cfg(test)]
mod tests;
