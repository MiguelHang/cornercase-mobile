use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::{self, BoxFuture, Either, Fuse};
use futures_util::{FutureExt, StreamExt, stream};
use tokio::sync::mpsc;
use tokio::time;

use super::grid::{Applied, Changes, Grid};
use super::shown::{Shown, Update};
use crate::cli::status::Snapshot;
use crate::cli::surface::{Frame, Size};
use crate::error::Error;
use crate::queue::Queue;
use crate::store::{Connection, Store};
use crate::transport::{Exit, Output, Transport, json_error, listen, only_stdout, printed};

const REDRAW: &str = "redraw";
const SETTLE: Duration = Duration::from_secs(1);

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

pub struct Live {
    queue: Queue,
    grid: Grid,
    redrawing: bool,
    lines: mpsc::UnboundedReceiver<String>,
    run: Fuse<BoxFuture<'static, End>>,
    ended: Option<Ended>,
}

impl Live {
    #[must_use]
    pub fn open(transport: Arc<dyn Transport>, queue: Queue, tab: u64, size: Size) -> Self {
        let (out, lines) = listen(only_stdout);
        let argv = surface(tab, size);
        let run = async move { transport.run(argv, stream::pending().boxed(), out).await }.boxed().fuse();
        Self { queue, grid: Grid::default(), redrawing: false, lines, run, ended: None }
    }

    #[must_use]
    pub fn grid(&self) -> &Grid {
        &self.grid
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
        let complete = frame.base.is_none();
        if self.redrawing && !complete {
            return None;
        }
        let surface = frame.surface;
        if self.grid.apply(frame) == Applied::NeedRedraw {
            self.redraw(surface);
            return None;
        }
        self.redrawing = false;
        Some(Step::Changed(self.grid.changed_rows()))
    }

    fn redraw(&mut self, surface: u64) {
        self.redrawing = true;
        self.queue.coalesced(REDRAW, vec![REDRAW.to_owned(), "--surface".to_owned(), surface.to_string()]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Update(Update),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminal {
    pub tab: u64,
    pub pane: Option<u64>,
    pub size: Size,
}

impl Terminal {
    pub async fn run(self, store: Store, mut sink: impl FnMut(Screen) + Send) {
        while let Some(reached) = reached(&store, self.tab).await {
            if !reached.open {
                sink(Screen::Closed);
                return;
            }
            self.follow(Live::open(reached.transport, reached.queue, self.tab, self.size), &mut sink).await;
            time::sleep(SETTLE).await;
        }
    }

    async fn follow(self, mut live: Live, sink: &mut impl FnMut(Screen)) {
        let mut shown = Shown::new(self.pane);
        loop {
            let changes = match live.next().await {
                Step::Changed(changes) => changes,
                Step::Warned(_) => continue,
                Step::Ended(_) => return,
            };
            if let Some(update) = shown.update(live.grid(), &changes) {
                sink(Screen::Update(update));
            }
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

fn surface(tab: u64, size: Size) -> Vec<String> {
    let (tab, cols, rows) = (tab.to_string(), size.cols.to_string(), size.rows.to_string());
    ["surface", "--tab", &tab, "--cols", &cols, "--rows", &rows, "--patches"].map(str::to_owned).to_vec()
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
