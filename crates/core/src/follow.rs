use std::marker::PhantomData;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::{self, BoxFuture, Either, Fuse};
use futures_util::{FutureExt, StreamExt, stream};
use tokio::sync::mpsc;
use tokio::time;

use crate::cli::status::Snapshot;
use crate::error::Error;
use crate::queue::Queue;
use crate::store::{Connection, Store};
use crate::transport::{Exit, Output, Transport, listen, only_stdout, printed};

const SETTLE: Duration = Duration::from_secs(1);

pub(crate) type End = Result<Exit, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    Lost(Error),
    Exited,
    Failed(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step<T> {
    Changed(T),
    Warned(Error),
    Ended(Ended),
}

pub(crate) struct Lines {
    stdout: mpsc::UnboundedReceiver<String>,
    run: Fuse<BoxFuture<'static, End>>,
    ended: Option<Ended>,
}

impl Lines {
    pub(crate) fn open(transport: Arc<dyn Transport>, argv: Vec<String>) -> Self {
        let (out, stdout) = listen(only_stdout);
        let run = async move { transport.run(argv, stream::pending().boxed(), out).await }.boxed().fuse();
        Self { stdout, run, ended: None }
    }

    pub(crate) async fn next<T>(&mut self, mut applied: impl FnMut(&str) -> Option<Step<T>>) -> Step<T> {
        loop {
            let line = match self.heard().await {
                Either::Left(line) => line,
                Either::Right(ended) => return Step::Ended(ended),
            };
            if let Some(step) = applied(&line) {
                return step;
            }
        }
    }

    async fn heard(&mut self) -> Either<String, Ended> {
        if self.ended.is_none() {
            let end = match future::select(pin!(self.stdout.recv()), &mut self.run).await {
                Either::Left((Some(line), _)) => return Either::Left(line),
                Either::Left((None, run)) => run.await,
                Either::Right((end, _)) => end,
            };
            self.ended = Some(ended(end));
        }
        match self.stdout.try_recv() {
            Ok(line) => Either::Left(line),
            Err(_) => Either::Right(self.ended.clone().unwrap_or(Ended::Exited)),
        }
    }
}

pub(crate) trait Notice {
    fn failed(error: Error) -> Self;
    fn closed() -> Self;
}

pub(crate) trait Opened {
    type Said: Notice;

    fn next(&mut self) -> impl Future<Output = Step<Option<Self::Said>>> + Send;
}

pub(crate) struct Reached {
    pub(crate) transport: Arc<dyn Transport>,
    pub(crate) queue: Queue,
    open: bool,
}

impl Reached {
    fn now(store: &Store, holds: &impl Fn(&Snapshot) -> bool) -> Option<Self> {
        if !matches!(store.connection(), Connection::Connected { .. }) {
            return None;
        }
        let open = holds(&store.session()?);
        Some(Self { transport: store.transport()?, queue: store.queue()?, open })
    }
}

pub(crate) async fn reopen<O: Opened>(
    store: &Store,
    holds: impl Fn(&Snapshot) -> bool,
    mut open: impl FnMut(Reached) -> O,
    sink: impl FnMut(O::Said),
) {
    let mut told = Told { sink, failing: false, said: PhantomData };
    while let Some(reached) = reached(store, &holds).await {
        if !reached.open {
            told.closed();
            return;
        }
        let used = reached.transport.clone();
        follow(store, &used, open(reached), &mut told).await;
        time::sleep(SETTLE).await;
    }
}

async fn follow<O: Opened>(
    store: &Store,
    used: &Arc<dyn Transport>,
    mut opened: O,
    told: &mut Told<O::Said, impl FnMut(O::Said)>,
) {
    let mut moved = pin!(moved_on(store, used));
    loop {
        let step = match future::select(pin!(opened.next()), moved.as_mut()).await {
            Either::Left((step, _)) => step,
            Either::Right(_) => return,
        };
        match step {
            Step::Changed(said) => told.said(said),
            Step::Warned(error) => told.failed(error),
            Step::Ended(ended) => {
                told.ended(ended, still_on(store, used));
                return;
            }
        }
    }
}

struct Told<T, S> {
    sink: S,
    failing: bool,
    said: PhantomData<fn(T)>,
}

impl<T: Notice, S: FnMut(T)> Told<T, S> {
    fn said(&mut self, said: Option<T>) {
        self.failing = false;
        if let Some(said) = said {
            (self.sink)(said);
        }
    }

    fn closed(&mut self) {
        (self.sink)(T::closed());
    }

    fn failed(&mut self, error: Error) {
        if !self.failing {
            self.failing = true;
            (self.sink)(T::failed(error));
        }
    }

    fn ended(&mut self, ended: Ended, store_still_on_it: bool) {
        match ended {
            Ended::Failed(error) => self.failed(error),
            Ended::Lost(error) if store_still_on_it => self.failed(error),
            Ended::Lost(_) | Ended::Exited => {}
        }
    }
}

fn still_on(store: &Store, used: &Arc<dyn Transport>) -> bool {
    let connected = matches!(store.connection(), Connection::Connected { .. });
    connected && store.transport().is_some_and(|now| Arc::ptr_eq(&now, used))
}

async fn moved_on(store: &Store, used: &Arc<dyn Transport>) {
    let mut changes = store.changes();
    while still_on(store, used) {
        if changes.next().await.is_none() {
            return;
        }
    }
}

async fn reached(store: &Store, holds: &impl Fn(&Snapshot) -> bool) -> Option<Reached> {
    let mut changes = store.changes();
    loop {
        if let Some(reached) = Reached::now(store, holds) {
            return Some(reached);
        }
        changes.next().await?;
    }
}

fn ended(end: End) -> Ended {
    match end {
        Ok(Exit { code: 0, .. }) => Ended::Exited,
        Ok(exit) => Ended::Failed(failed(exit)),
        Err(error) => Ended::Lost(error),
    }
}

fn failed(exit: Exit) -> Error {
    let output = Output { stdout: String::new(), stderr: exit.stderr, code: exit.code };
    printed(output).err().unwrap_or(Error::StreamEnded)
}
