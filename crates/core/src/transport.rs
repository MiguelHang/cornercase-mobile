use std::iter;
use std::pin::Pin;

use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;
use futures_util::{Sink, StreamExt, sink, stream};
use serde::de::DeserializeOwned;
use serde_json::error::Category;
use tokio::sync::mpsc;

use crate::error::Error;

pub mod bridge;
#[cfg(test)]
pub(crate) mod fake;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stdin {
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    Stdout(String),
    Stderr(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exit {
    pub code: i32,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Closed;

pub type OutSink = Pin<Box<dyn Sink<Out, Error = Closed> + Send>>;

pub trait Transport: Send + Sync {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>>;
}

pub async fn output(transport: &dyn Transport, argv: Vec<String>) -> Result<Output, Error> {
    let (out, mut receiver) = listen();
    let exit = transport.run(argv, stream::empty().boxed(), out).await?;
    let stdout: Vec<String> = iter::from_fn(|| receiver.try_recv().ok())
        .filter_map(|out| match out {
            Out::Stdout(line) => Some(line),
            Out::Stderr(_) => None,
        })
        .collect();
    Ok(Output { stdout: stdout.join("\n"), stderr: exit.stderr, code: exit.code })
}

pub async fn json<T: DeserializeOwned>(transport: &dyn Transport, argv: Vec<String>) -> Result<T, Error> {
    let output = output(transport, argv).await?;
    if output.code != 0 {
        return Err(Error::Failed { code: output.code, stderr: output.stderr });
    }
    serde_json::from_str(&output.stdout).map_err(|error| json_error(&error))
}

pub(crate) fn listen() -> (OutSink, mpsc::UnboundedReceiver<Out>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let sink = sink::unfold(sender, async |sender, out| {
        sender.send(out).map_err(|_| Closed)?;
        Ok(sender)
    });
    (Box::pin(sink), receiver)
}

fn json_error(error: &serde_json::Error) -> Error {
    let reason = match error.classify() {
        Category::Syntax | Category::Io => "not JSON",
        Category::Eof => "the JSON ends too early",
        Category::Data => "not the expected shape",
    };
    Error::Json(format!("{reason} at line {} column {}", error.line(), error.column()))
}

#[derive(Debug, Default)]
pub(crate) struct StderrTail {
    text: String,
    started: bool,
}

impl StderrTail {
    pub(crate) const LIMIT: usize = 64 << 10;

    pub(crate) fn push(&mut self, line: &str) {
        if self.started {
            self.text.push('\n');
        }
        self.started = true;
        self.text.push_str(line);
        if self.text.len() > 2 * Self::LIMIT {
            self.keep_the_end();
        }
    }

    pub(crate) fn finish(mut self) -> String {
        self.keep_the_end();
        self.text
    }

    fn keep_the_end(&mut self) {
        let excess = self.text.len().saturating_sub(Self::LIMIT);
        let start = self.text.ceil_char_boundary(excess);
        self.text.drain(..start);
    }
}

#[cfg(test)]
mod tests;
