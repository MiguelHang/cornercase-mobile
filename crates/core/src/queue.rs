use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use self::worker::{Command, Job, Worker};
use crate::error::Error;
use crate::transport::Transport;

mod follow;
mod requests;
mod typist;
mod worker;

pub(crate) type OnError = Arc<dyn Fn(Error) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done(String),
}

#[derive(Clone)]
pub struct Queue {
    jobs: mpsc::UnboundedSender<Job>,
}

impl Queue {
    pub fn new(
        transport: Arc<dyn Transport>,
        on_error: impl Fn(Error) + Send + Sync + 'static,
    ) -> (Self, impl Future<Output = ()> + Send + 'static) {
        let (jobs, waiting) = mpsc::unbounded_channel();
        (Self { jobs }, Worker::new(transport, Arc::new(on_error), waiting).work())
    }

    pub fn run(&self, args: Vec<String>, stdin: Option<String>) {
        self.push(Job::Command(Command { args, stdin, reply: None }));
    }

    pub fn ask(
        &self,
        args: Vec<String>,
        stdin: Option<String>,
    ) -> impl Future<Output = Result<Outcome, Error>> + Send + 'static {
        let (reply, answer) = oneshot::channel();
        self.push(Job::Command(Command { args, stdin, reply: Some(reply) }));
        async move { answer.await.unwrap_or(Err(Error::Unanswered)) }
    }

    pub fn input(&self, pane: u64, event: Value) {
        self.push(Job::Input((pane, event)));
    }

    fn push(&self, job: Job) {
        let _ = self.jobs.send(job);
    }
}

#[cfg(test)]
mod tests;
