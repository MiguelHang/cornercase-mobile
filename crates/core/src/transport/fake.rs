use std::collections::HashMap;
use std::future::{self, Future};

use futures_util::future::{BoxFuture, FutureExt};
use futures_util::stream::BoxStream;
use futures_util::{SinkExt, stream};

use super::{Closed, Exit, Out, OutSink, StderrTail, Stdin, Transport};
use crate::error::Error;

type Script = Box<dyn Fn(BoxStream<'static, Stdin>, OutSink) -> BoxFuture<'static, Result<Exit, Error>> + Send + Sync>;

#[derive(Default)]
pub(crate) struct FakeTransport {
    scripts: HashMap<Vec<String>, Script>,
}

impl FakeTransport {
    pub(crate) fn answer(self, argv: &[&str], stdout: &[&str], stderr: &[&str], code: i32) -> Self {
        let lines: Vec<Out> =
            owned(stdout).into_iter().map(Out::Stdout).chain(owned(stderr).into_iter().map(Out::Stderr)).collect();
        let mut tail = StderrTail::default();
        for line in stderr {
            tail.push(line);
        }
        let stderr = tail.finish();
        self.script(argv, move |_, mut out| {
            let lines = lines.clone();
            let stderr = stderr.clone();
            async move {
                let mut lines = stream::iter(lines.into_iter().map(Ok));
                out.send_all(&mut lines).await.map_err(|Closed| Error::OutputClosed)?;
                Ok(Exit { code, stderr })
            }
        })
    }

    pub(crate) fn fail(self, argv: &[&str], error: Error) -> Self {
        self.script(argv, move |_, _| future::ready(Err(error.clone())))
    }

    pub(crate) fn script<S, F>(mut self, argv: &[&str], script: S) -> Self
    where
        S: Fn(BoxStream<'static, Stdin>, OutSink) -> F + Send + Sync + 'static,
        F: Future<Output = Result<Exit, Error>> + Send + 'static,
    {
        self.scripts.insert(owned(argv), Box::new(move |stdin, out| script(stdin, out).boxed()));
        self
    }
}

impl Transport for FakeTransport {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>> {
        match self.scripts.get(&argv) {
            Some(script) => script(stdin, out),
            None => future::ready(Err(Error::Unscripted(argv))).boxed(),
        }
    }
}

fn owned(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

#[cfg(test)]
mod tests;
