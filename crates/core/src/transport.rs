use std::pin::Pin;

use futures_util::Sink;
use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;

use crate::error::Error;

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

#[cfg_attr(not(test), expect(dead_code, reason = "the bridge transport collects its stderr with it"))]
#[derive(Debug, Default)]
pub(crate) struct StderrTail {
    text: String,
    started: bool,
}

#[cfg_attr(not(test), expect(dead_code, reason = "the bridge transport collects its stderr with it"))]
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
