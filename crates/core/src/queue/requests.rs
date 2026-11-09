use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, BoxFuture, Either, Fuse, FusedFuture};
use futures_util::{FutureExt, StreamExt, stream};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::error::Error;
use crate::protocol::USAGE_EXIT;
use crate::transport::{Exit, STALE_BOOT, Stdin, Transport, json_error, listen, only_stdout};

const FOLLOW: [&str; 3] = ["requests", "--follow", "-"];
const NO_REQUESTS: &str = "unrecognized subcommand 'requests'";
const NO_FOLLOW: &str = "unexpected argument '--follow'";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Answer {
    status: i32,
    json: Option<Value>,
    text: String,
    stderr: String,
    error: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Asked {
    Answered(Result<String, Error>),
    Unsent,
    Absent,
}

type End = Result<Exit, Error>;

pub(crate) struct Requests {
    lines: mpsc::UnboundedSender<Stdin>,
    answers: mpsc::UnboundedReceiver<String>,
    run: Fuse<BoxFuture<'static, End>>,
    answered: bool,
}

impl Requests {
    pub(crate) fn start(transport: Arc<dyn Transport>) -> Self {
        let (lines, mut stdin) = mpsc::unbounded_channel();
        let stdin = stream::poll_fn(move |context| stdin.poll_recv(context)).boxed();
        let (out, answers) = listen(only_stdout);
        let argv = FOLLOW.map(str::to_owned).to_vec();
        let run = async move { transport.run(argv, stdin, out).await }.boxed().fuse();
        Self { lines, answers, run, answered: false }
    }

    pub(crate) async fn ask(&mut self, line: &str) -> Asked {
        if self.run.is_terminated() {
            return Asked::Unsent;
        }
        if let Some(end) = (&mut self.run).now_or_never() {
            return self.unsent(&end);
        }
        if self.lines.send(Stdin::Text(format!("{line}\n"))).is_err() {
            let end = (&mut self.run).await;
            return self.unsent(&end);
        }
        match self.next().await {
            Either::Left(Some(line)) => self.answer(&line),
            Either::Left(None) => {
                let end = (&mut self.run).await;
                self.lost(&end)
            }
            Either::Right(end) => match self.answers.try_recv() {
                Ok(line) => self.answer(&line),
                Err(_) => self.lost(&end),
            },
        }
    }

    pub(crate) async fn ended(&mut self) {
        if !self.run.is_terminated() {
            let _ = (&mut self.run).await;
        }
    }

    async fn next(&mut self) -> Either<Option<String>, End> {
        let answer = pin!(self.answers.recv());
        match future::select(answer, &mut self.run).await {
            Either::Left((line, _)) => Either::Left(line),
            Either::Right((end, _)) => Either::Right(end),
        }
    }

    fn answer(&mut self, line: &str) -> Asked {
        self.answered = true;
        Asked::Answered(answer(line))
    }

    fn unsent(&self, end: &End) -> Asked {
        match self.lost(end) {
            Asked::Answered(_) => Asked::Unsent,
            asked => asked,
        }
    }

    fn lost(&self, end: &End) -> Asked {
        match end {
            Ok(exit) if !self.answered && absent(exit) => Asked::Absent,
            Err(error) if unopened(error) => Asked::Unsent,
            _ => Asked::Answered(Err(Error::Unanswered)),
        }
    }
}

pub(crate) fn line(args: &[String], stdin: Option<&str>) -> String {
    let mut line = json!({ "args": args });
    if let Some(text) = stdin {
        line["stdin"] = Value::from(text);
    }
    line.to_string()
}

pub(crate) fn answer(line: &str) -> Result<String, Error> {
    let answer: Answer = serde_json::from_str(line).map_err(|error| json_error(&error))?;
    match answer.status {
        0 => Ok(printed(answer)),
        STALE_BOOT => Err(Error::StaleBoot(why(&answer))),
        code => Err(Error::Failed { code, stderr: why(&answer) }),
    }
}

pub(crate) fn absent(exit: &Exit) -> bool {
    exit.code == USAGE_EXIT && [NO_REQUESTS, NO_FOLLOW].iter().any(|refusal| exit.stderr.contains(refusal))
}

fn unopened(error: &Error) -> bool {
    matches!(error, Error::Unauthorized | Error::Rejected { .. } | Error::Timeout | Error::Connect(_))
}

fn printed(answer: Answer) -> String {
    let text = answer.text.strip_suffix('\n').unwrap_or(&answer.text);
    match answer.json {
        Some(json) if answer.text.is_empty() => json.to_string(),
        Some(json) => format!("{json}\n{text}"),
        None => text.to_owned(),
    }
}

fn why(answer: &Answer) -> String {
    let error = answer.error.trim_end();
    let why = if error.is_empty() { answer.stderr.trim_end() } else { error };
    why.to_owned()
}

#[cfg(test)]
mod tests;
