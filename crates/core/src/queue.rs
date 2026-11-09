use std::future::Future;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use self::worker::{Command, Job, Worker};
use crate::error::Error;
use crate::transport::Transport;

mod follow;
mod requests;
#[cfg_attr(not(test), expect(dead_code, reason = "only the tests type through it until the worker sends input"))]
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
        self.push(Command { args, stdin, reply: None });
    }

    pub fn ask(
        &self,
        args: Vec<String>,
        stdin: Option<String>,
    ) -> impl Future<Output = Result<Outcome, Error>> + Send + 'static {
        let (reply, answer) = oneshot::channel();
        self.push(Command { args, stdin, reply: Some(reply) });
        async move { answer.await.unwrap_or(Err(Error::Unanswered)) }
    }

    fn push(&self, command: Command) {
        let _ = self.jobs.send(Job::Command(command));
    }
}

#[cfg(test)]
mod tests;
