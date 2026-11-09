use std::future;
use std::iter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::stream::BoxStream;
use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use serde_json::{Value, json};
use tokio::task::JoinHandle;
use tokio::time;

use super::*;
use crate::transport::fake::FakeTransport;
use crate::transport::{Closed, Exit, Out, OutSink, Stdin};

mod commands;
mod input;

const REQUESTS: [&str; 3] = ["requests", "--follow", "-"];
const NO_REQUESTS_COMMAND: &str = "error: unrecognized subcommand 'requests'\n\nUsage: cornercase [OPTIONS]";

type End = Result<Exit, Error>;

struct Heard<T>(mpsc::UnboundedReceiver<T>);

impl<T> Heard<T> {
    fn taken(&mut self) -> Vec<T> {
        iter::from_fn(|| self.0.try_recv().ok()).collect()
    }
}

fn heard<T>() -> (mpsc::UnboundedSender<T>, Heard<T>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    (sender, Heard(receiver))
}

#[derive(Clone, Default)]
struct Starts(Arc<AtomicUsize>);

impl Starts {
    fn add(&self) -> usize {
        self.0.fetch_add(1, Ordering::SeqCst)
    }

    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

struct Started {
    queue: Queue,
    notices: Heard<Error>,
    worker: JoinHandle<()>,
}

fn start(fake: FakeTransport) -> Started {
    let (notice, notices) = heard();
    let on_error = move |error| notice.send(error).expect("the test reads the notices");
    let (queue, worker) = Queue::new(Arc::new(fake), on_error);
    Started { queue, notices, worker: tokio::spawn(worker) }
}

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

fn done(printed: &str) -> Outcome {
    Outcome::Done(printed.to_owned())
}

fn it_failed() -> Error {
    Error::Failed { code: 1, stderr: "it failed".to_owned() }
}

async fn settled() {
    time::sleep(Duration::from_millis(1)).await;
}

fn answering(fake: FakeTransport, log: &mpsc::UnboundedSender<String>, starts: &Starts) -> FakeTransport {
    let (log, starts) = (log.clone(), starts.clone());
    fake.script(&REQUESTS, move |stdin, out| {
        starts.add();
        answer_lines(log.clone(), stdin, out)
    })
}

fn refusing(fake: FakeTransport, end: End, starts: &Starts) -> FakeTransport {
    let starts = starts.clone();
    fake.script(&REQUESTS, move |_, _| {
        starts.add();
        future::ready(end.clone())
    })
}

fn reading(fake: FakeTransport, argv: &[&str]) -> FakeTransport {
    fake.script(argv, async |stdin, mut out| {
        let read: Vec<String> = stdin.filter_map(async |stdin| text(stdin)).collect().await;
        let printed = format!("once read {}", read.concat());
        out.send(Out::Stdout(printed)).await.map_err(|Closed| Error::OutputClosed)?;
        Ok(exit(0, ""))
    })
}

fn text(stdin: Stdin) -> Option<String> {
    match stdin {
        Stdin::Text(text) => Some(text),
        Stdin::Bytes(_) => None,
    }
}

async fn answer_lines(
    log: mpsc::UnboundedSender<String>,
    mut stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> End {
    while let Some(Stdin::Text(line)) = stdin.next().await {
        say(&log, &mut out, &line).await?;
    }
    Ok(exit(0, ""))
}

async fn say(log: &mpsc::UnboundedSender<String>, out: &mut OutSink, line: &str) -> Result<(), Error> {
    let line: Value = serde_json::from_str(line).expect("a requests line");
    let args: Vec<&str> = line["args"].as_array().expect("its args").iter().filter_map(Value::as_str).collect();
    let said = match line["stdin"].as_str() {
        Some(stdin) => format!("{} < {stdin}", args.join(" ")),
        None => args.join(" "),
    };
    log.send(said.clone()).expect("the test reads the log");
    let answer = match args.first() {
        Some(&"fail") => json!({ "status": 1, "error": "it failed" }),
        _ => json!({ "status": 0, "text": said }),
    };
    out.send(Out::Stdout(answer.to_string())).await.map_err(|Closed| Error::OutputClosed)
}
