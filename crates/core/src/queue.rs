use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use self::worker::{Command, Job, Worker};
use crate::error::Error;
use crate::transport::{Stdin, Transport};

mod batch;
mod follow;
mod requests;
mod typist;
mod worker;

pub(crate) type OnError = Arc<dyn Fn(Error) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done(String),
    Skipped,
}

#[derive(Clone)]
pub struct Queue {
    jobs: mpsc::UnboundedSender<Job>,
    control: mpsc::UnboundedSender<()>,
}

impl Queue {
    pub fn new(
        transport: Arc<dyn Transport>,
        on_error: impl Fn(Error) + Send + Sync + 'static,
    ) -> (Self, impl Future<Output = ()> + Send + 'static) {
        let (jobs, waiting) = mpsc::unbounded_channel();
        let (control, signals) = mpsc::unbounded_channel();
        (Self { jobs, control }, Worker::new(transport, Arc::new(on_error), waiting).work(signals))
    }

    pub fn run(&self, args: Vec<String>, stdin: Option<Stdin>) {
        self.push(Job::Command(Command { args, stdin, key: None, reply: None }));
    }

    pub fn ask(
        &self,
        args: Vec<String>,
        stdin: Option<Stdin>,
    ) -> impl Future<Output = Result<Outcome, Error>> + Send + 'static {
        self.asked(Command { args, stdin, key: None, reply: None })
    }

    pub fn coalesced(&self, key: &'static str, args: Vec<String>) {
        self.push(Job::Command(Command { args, stdin: None, key: Some(key), reply: None }));
    }

    pub fn ask_coalesced(
        &self,
        key: &'static str,
        args: Vec<String>,
    ) -> impl Future<Output = Result<Outcome, Error>> + Send + 'static {
        self.asked(Command { args, stdin: None, key: Some(key), reply: None })
    }

    pub fn scroll(&self, pane: u64, rows: i64) {
        self.push(Job::Scroll(pane, rows));
    }

    pub fn input(&self, pane: u64, event: Value) {
        self.push(Job::Input((pane, event)));
    }

    pub fn reconnect(&self, transport: Arc<dyn Transport>) {
        let _ = self.control.send(());
        self.push(Job::Reconnect(transport));
    }

    fn asked(&self, command: Command) -> impl Future<Output = Result<Outcome, Error>> + Send + 'static {
        let (reply, answer) = oneshot::channel();
        self.push(Job::Command(Command { reply: Some(reply), ..command }));
        async move { answer.await.unwrap_or(Err(Error::Unanswered)) }
    }

    fn push(&self, job: Job) {
        let _ = self.jobs.send(job);
    }
}

#[cfg(test)]
mod tests;
