use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, Either};
use futures_util::{StreamExt, stream};
use tokio::sync::{mpsc, oneshot};

use super::requests::{self, Asked, Requests};
use super::{OnError, Outcome};
use crate::error::Error;
use crate::transport::{self, Stdin, Transport};

pub(crate) type Reply = oneshot::Sender<Result<Outcome, Error>>;

pub(crate) enum Job {
    Command(Command),
}

pub(crate) struct Command {
    pub(crate) args: Vec<String>,
    pub(crate) stdin: Option<String>,
    pub(crate) reply: Option<Reply>,
}

pub(crate) struct Worker {
    transport: Arc<dyn Transport>,
    on_error: OnError,
    jobs: mpsc::UnboundedReceiver<Job>,
    requests: Option<Requests>,
    absent: bool,
}

impl Worker {
    pub(crate) fn new(transport: Arc<dyn Transport>, on_error: OnError, jobs: mpsc::UnboundedReceiver<Job>) -> Self {
        Self { transport, on_error, jobs, requests: None, absent: false }
    }

    pub(crate) async fn work(mut self) {
        while let Some(job) = self.next().await {
            match job {
                Job::Command(command) => self.command(command).await,
            }
        }
    }

    async fn next(&mut self) -> Option<Job> {
        while let Some(requests) = &mut self.requests {
            match waited(&mut self.jobs, requests).await {
                Either::Left(job) => return job,
                Either::Right(()) => self.requests = None,
            }
        }
        self.jobs.recv().await
    }

    async fn command(&mut self, command: Command) {
        let result = self.run(&command.args, command.stdin.as_deref()).await;
        self.reply(command.reply, result.map(Outcome::Done));
    }

    async fn run(&mut self, args: &[String], stdin: Option<&str>) -> Result<String, Error> {
        match self.ask(&requests::line(args, stdin)).await {
            Asked::Answered(result) => result,
            Asked::Unsent | Asked::Absent => once(self.transport.as_ref(), args, stdin).await,
        }
    }

    async fn ask(&mut self, line: &str) -> Asked {
        if self.absent {
            return Asked::Absent;
        }
        let requests = self.requests.get_or_insert_with(|| Requests::start(Arc::clone(&self.transport)));
        let asked = requests.ask(line).await;
        if requests.is_over() {
            self.requests = None;
        }
        self.absent = asked == Asked::Absent;
        asked
    }

    fn reply(&self, reply: Option<Reply>, result: Result<Outcome, Error>) {
        let unheard = match reply {
            Some(reply) => reply.send(result).err(),
            None => Some(result),
        };
        if let Some(Err(error)) = unheard {
            (self.on_error)(error);
        }
    }
}

async fn waited(jobs: &mut mpsc::UnboundedReceiver<Job>, requests: &mut Requests) -> Either<Option<Job>, ()> {
    let job = pin!(jobs.recv());
    let ended = pin!(requests.ended());
    match future::select(job, ended).await {
        Either::Left((job, _)) => Either::Left(job),
        Either::Right(((), _)) => Either::Right(()),
    }
}

async fn once(transport: &dyn Transport, args: &[String], stdin: Option<&str>) -> Result<String, Error> {
    let stdin = stream::iter(stdin.map(|text| Stdin::Text(text.to_owned()))).boxed();
    transport::printed(transport::fed(transport, args.to_vec(), stdin).await?)
}
