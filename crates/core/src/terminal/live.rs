use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::{self, BoxFuture, Either, Fuse};
use futures_util::{FutureExt, StreamExt, stream};
use tokio::sync::mpsc;
use tokio::time::{self, Instant};

use super::grid::{Applied, Changes, Grid};
use super::shown::{Shown, Update};
use crate::cli::status::Snapshot;
use crate::cli::surface::{Frame, Size, on_tab};
use crate::error::Error;
use crate::queue::Queue;
use crate::store::{Connection, Store};
use crate::transport::{Exit, Output, Transport, json_error, listen, only_stdout, printed};

const REDRAW: &str = "redraw";
const SETTLE: Duration = Duration::from_secs(1);
const REDRAW_AGAIN: Duration = Duration::from_secs(2);

type End = Result<Exit, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    Lost(Error),
    Exited,
    Failed(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Changed(Changes),
    Warned(Error),
    Ended(Ended),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Tab { tab: u64, size: Size },
    View { view: u64, tab: u64 },
}

impl Source {
    fn shows(self, frame: &Frame) -> bool {
        match self {
            Self::Tab { .. } => true,
            Self::View { view, tab } => frame.view == Some(view) && frame.tab == tab,
        }
    }
}

pub struct Live {
    source: Source,
    queue: Queue,
    grid: Grid,
    redrawing: Option<Instant>,
    lines: mpsc::UnboundedReceiver<String>,
    run: Fuse<BoxFuture<'static, End>>,
    ended: Option<Ended>,
}

impl Live {
    #[must_use]
    pub fn open(transport: Arc<dyn Transport>, queue: Queue, source: Source) -> Self {
        let (out, lines) = listen(only_stdout);
        let argv = surface(source);
        let run = async move { transport.run(argv, stream::pending().boxed(), out).await }.boxed().fuse();
        Self { source, queue, grid: Grid::default(), redrawing: None, lines, run, ended: None }
    }

    #[must_use]
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn saw_projection(&mut self, projection: u64) {
        self.grid.saw_projection(projection);
    }

    pub async fn next(&mut self) -> Step {
        loop {
            let line = match self.heard().await {
                Either::Left(line) => line,
                Either::Right(ended) => return Step::Ended(ended),
            };
            if let Some(step) = self.applied(&line) {
                return step;
            }
        }
    }

    async fn heard(&mut self) -> Either<String, Ended> {
        if self.ended.is_none() {
            let end = match future::select(pin!(self.lines.recv()), &mut self.run).await {
                Either::Left((Some(line), _)) => return Either::Left(line),
                Either::Left((None, run)) => run.await,
                Either::Right((end, _)) => end,
            };
            self.ended = Some(ended(end));
        }
        match self.lines.try_recv() {
            Ok(line) => Either::Left(line),
            Err(_) => Either::Right(self.ended.clone().unwrap_or(Ended::Exited)),
        }
    }

    fn applied(&mut self, line: &str) -> Option<Step> {
        let frame: Frame = match serde_json::from_str(line) {
            Ok(frame) => frame,
            Err(error) => return Some(Step::Warned(json_error(&error))),
        };
        if !self.source.shows(&frame) {
            return None;
        }
        let surface = frame.surface;
        if frame.base.is_some() && self.still_redrawing(surface) {
            return None;
        }
        if self.grid.apply(frame) == Applied::NeedRedraw {
            self.redraw(surface);
            return None;
        }
        self.redrawing = None;
        let changes = self.grid.changed_rows();
        self.grid.is_current().then_some(Step::Changed(changes))
    }

    fn still_redrawing(&mut self, surface: u64) -> bool {
        let Some(asked) = self.redrawing else {
            return false;
        };
        if asked.elapsed() >= REDRAW_AGAIN {
            self.redraw(surface);
        }
        true
    }

    fn redraw(&mut self, surface: u64) {
        self.redrawing = Some(Instant::now());
        self.queue.coalesced(REDRAW, vec![REDRAW.to_owned(), "--surface".to_owned(), surface.to_string()]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Update(Update),
    Failed(Error),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminal {
    pub tab: u64,
    pub pane: Option<u64>,
    pub size: Size,
}

impl Terminal {
    pub async fn run(self, store: Store, sink: impl FnMut(Screen) + Send) {
        let mut told = Told { sink, failing: false };
        while let Some(reached) = reached(&store, self.tab).await {
            if !reached.open {
                told.closed();
                return;
            }
            self.follow(&store, reached, &mut told).await;
            time::sleep(SETTLE).await;
        }
    }

    async fn follow(self, store: &Store, reached: Reached, told: &mut Told<impl FnMut(Screen)>) {
        let used = reached.transport.clone();
        let source = Source::Tab { tab: self.tab, size: self.size };
        let mut live = Live::open(reached.transport, reached.queue, source);
        let mut moved = pin!(moved_on(store, &used));
        let mut shown = Shown::new(self.pane);
        loop {
            let step = match future::select(pin!(live.next()), moved.as_mut()).await {
                Either::Left((step, _)) => step,
                Either::Right(_) => return,
            };
            match step {
                Step::Changed(changes) => told.shown(shown.update(live.grid(), &changes)),
                Step::Warned(error) => told.failed(error),
                Step::Ended(ended) => {
                    told.ended(ended, still_on(store, &used));
                    return;
                }
            }
        }
    }
}

struct Told<S> {
    sink: S,
    failing: bool,
}

impl<S: FnMut(Screen)> Told<S> {
    fn shown(&mut self, update: Option<Update>) {
        self.failing = false;
        if let Some(update) = update {
            (self.sink)(Screen::Update(update));
        }
    }

    fn closed(&mut self) {
        (self.sink)(Screen::Closed);
    }

    fn failed(&mut self, error: Error) {
        if !self.failing {
            self.failing = true;
            (self.sink)(Screen::Failed(error));
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

struct Reached {
    transport: Arc<dyn Transport>,
    queue: Queue,
    open: bool,
}

impl Reached {
    fn now(store: &Store, tab: u64) -> Option<Self> {
        if !matches!(store.connection(), Connection::Connected { .. }) {
            return None;
        }
        let open = holds(&store.session()?, tab);
        Some(Self { transport: store.transport()?, queue: store.queue()?, open })
    }
}

async fn reached(store: &Store, tab: u64) -> Option<Reached> {
    let mut changes = store.changes();
    loop {
        if let Some(reached) = Reached::now(store, tab) {
            return Some(reached);
        }
        changes.next().await?;
    }
}

fn holds(session: &Snapshot, tab: u64) -> bool {
    let workspaces = session.projects.iter().flat_map(|project| &project.workspaces);
    workspaces.flat_map(|workspace| &workspace.tabs).any(|open| open.id == tab)
}

fn surface(source: Source) -> Vec<String> {
    let mut argv = vec!["surface".to_owned()];
    match source {
        Source::Tab { tab, size } => argv.extend(on_tab(tab, size)),
        Source::View { view, .. } => argv.extend(["--view".to_owned(), view.to_string()]),
    }
    argv.push("--patches".to_owned());
    argv
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

#[cfg(test)]
mod tests;
