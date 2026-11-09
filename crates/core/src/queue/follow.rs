use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, BoxFuture, Either, Fuse, FusedFuture};
use futures_util::{FutureExt, StreamExt, stream};
use tokio::sync::mpsc;

use crate::error::Error;
use crate::protocol::USAGE_EXIT;
use crate::transport::{Exit, Stdin, Transport, listen, only_stdout};

const NO_REQUESTS: &str = "unrecognized subcommand 'requests'";
const NO_FOLLOW: &str = "unexpected argument '--follow'";

pub(crate) type End = Result<Exit, Error>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Gone {
    Absent,
    Unopened,
    Lost,
}

pub(crate) struct Follow {
    lines: mpsc::UnboundedSender<Stdin>,
    answers: mpsc::UnboundedReceiver<String>,
    run: Fuse<BoxFuture<'static, End>>,
    answered: bool,
}

impl Follow {
    pub(crate) fn start(transport: Arc<dyn Transport>, argv: &[&str]) -> Self {
        let (lines, mut stdin) = mpsc::unbounded_channel();
        let stdin = stream::poll_fn(move |context| stdin.poll_recv(context)).boxed();
        let (out, answers) = listen(only_stdout);
        let argv = argv.iter().map(|&word| word.to_owned()).collect();
        let run = async move { transport.run(argv, stdin, out).await }.boxed().fuse();
        Self { lines, answers, run, answered: false }
    }

    pub(crate) async fn write(&mut self, line: &str) -> Result<(), Option<End>> {
        if self.is_over() {
            return Err(None);
        }
        if let Some(end) = (&mut self.run).now_or_never() {
            return Err(Some(end));
        }
        if self.lines.send(Stdin::Text(format!("{line}\n"))).is_err() {
            return Err(Some((&mut self.run).await));
        }
        Ok(())
    }

    pub(crate) async fn next(&mut self) -> Either<String, End> {
        let answer = pin!(self.answers.recv());
        let heard = match future::select(answer, &mut self.run).await {
            Either::Left((line, _)) => Either::Left(line),
            Either::Right((end, _)) => Either::Right(end),
        };
        match heard {
            Either::Left(Some(line)) => {
                self.answered = true;
                Either::Left(line)
            }
            Either::Left(None) => Either::Right((&mut self.run).await),
            Either::Right(end) => Either::Right(end),
        }
    }

    pub(crate) fn ready(&mut self) -> Option<String> {
        let line = self.answers.try_recv().ok();
        self.answered |= line.is_some();
        line
    }

    pub(crate) fn answered(&self) -> bool {
        self.answered
    }

    pub(crate) fn is_over(&self) -> bool {
        self.run.is_terminated()
    }

    pub(crate) async fn end(&mut self) -> Option<End> {
        if self.is_over() { None } else { Some((&mut self.run).await) }
    }

    pub(crate) fn gone(&self, end: &End) -> Gone {
        match end {
            Ok(exit) if !self.answered && absent(exit) => Gone::Absent,
            Err(error) if unopened(error) => Gone::Unopened,
            _ => Gone::Lost,
        }
    }
}

pub(crate) fn absent(exit: &Exit) -> bool {
    exit.code == USAGE_EXIT && [NO_REQUESTS, NO_FOLLOW].iter().any(|refusal| exit.stderr.contains(refusal))
}

pub(crate) fn stopped(end: &End, ended: &str) -> Error {
    let exit = match end {
        Ok(exit) => exit,
        Err(error) => return error.clone(),
    };
    let why = exit.stderr.lines().map(str::trim).rfind(|line| !line.is_empty());
    Error::Failed { code: exit.code, stderr: why.unwrap_or(ended).to_owned() }
}

fn unopened(error: &Error) -> bool {
    matches!(error, Error::Unauthorized | Error::Rejected { .. } | Error::Timeout | Error::Connect(_))
}

#[cfg(test)]
mod tests;
