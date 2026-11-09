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
#[cfg(any(test, feature = "test-util"))]
pub mod fake;

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

pub(crate) const STALE_BOOT: i32 = 4;

pub async fn output(transport: &dyn Transport, argv: Vec<String>) -> Result<Output, Error> {
    fed(transport, argv, stream::empty().boxed()).await
}

pub async fn json<T: DeserializeOwned>(transport: &dyn Transport, argv: Vec<String>) -> Result<T, Error> {
    let stdout = printed(output(transport, argv).await?)?;
    serde_json::from_str(&stdout).map_err(|error| json_error(&error))
}

pub(crate) async fn fed(
    transport: &dyn Transport,
    argv: Vec<String>,
    stdin: BoxStream<'static, Stdin>,
) -> Result<Output, Error> {
    let (out, mut receiver) = listen(only_stdout);
    let exit = transport.run(argv, stdin, out).await?;
    let stdout: Vec<String> = iter::from_fn(|| receiver.try_recv().ok()).collect();
    Ok(Output { stdout: stdout.join("\n"), stderr: exit.stderr, code: exit.code })
}

pub(crate) fn printed(output: Output) -> Result<String, Error> {
    match output.code {
        0 => Ok(output.stdout),
        STALE_BOOT => Err(Error::StaleBoot(output.stderr)),
        code => Err(Error::Failed { code, stderr: output.stderr }),
    }
}

pub(crate) fn listen<T: Send + 'static>(keep: fn(Out) -> Option<T>) -> (OutSink, mpsc::UnboundedReceiver<T>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let sink = sink::unfold((sender, keep), async |(sender, keep), out| {
        if let Some(kept) = keep(out) {
            sender.send(kept).map_err(|_| Closed)?;
        }
        Ok((sender, keep))
    });
    (Box::pin(sink), receiver)
}

pub(crate) fn only_stdout(out: Out) -> Option<String> {
    match out {
        Out::Stdout(line) => Some(line),
        Out::Stderr(_) => None,
    }
}

pub(crate) fn json_error(error: &serde_json::Error) -> Error {
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
