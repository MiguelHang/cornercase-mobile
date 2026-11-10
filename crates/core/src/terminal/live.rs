use std::sync::Arc;
use std::time::Duration;

use tokio::time::{self, Instant};

use super::fit::{Claim, Fitted, follow_fitted};
use super::grid::{Applied, Changes, Grid};
use super::shown::{Shown, Update};
use crate::cli::status::Snapshot;
use crate::cli::surface::{Frame, Size, on_tab};
use crate::error::Error;
use crate::follow::{self, After, Lines, Notice, Opened, Reached, Told, Visit};
use crate::queue::Queue;
use crate::store::Store;
use crate::transport::{Transport, json_error};

const REDRAW: &str = "redraw";
const REDRAW_AGAIN: Duration = Duration::from_secs(2);
const LEAVING: Duration = Duration::from_secs(5);

pub type Step = follow::Step<Changes>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
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

pub(crate) struct Live {
    lines: Lines,
    drawing: Drawing,
}

struct Drawing {
    source: Source,
    queue: Queue,
    grid: Grid,
    redrawing: Option<Instant>,
}

impl Live {
    pub(crate) fn open(transport: Arc<dyn Transport>, queue: Queue, source: Source) -> Self {
        let lines = Lines::open(transport, surface(source));
        Self { lines, drawing: Drawing { source, queue, grid: Grid::default(), redrawing: None } }
    }

    pub(crate) fn grid(&self) -> &Grid {
        &self.drawing.grid
    }

    pub(crate) fn saw_projection(&mut self, projection: u64) {
        self.drawing.grid.saw_projection(projection);
    }

    pub(crate) async fn next(&mut self) -> Step {
        self.lines.next(|line| self.drawing.applied(line)).await
    }
}

impl Drawing {
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
pub enum Unfitted {
    TakenBack,
    Refused(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Update(Update),
    Failed(Error),
    Closed,
    Fit(Unfitted),
}

impl Notice for Screen {
    fn failed(error: Error) -> Self {
        Self::Failed(error)
    }

    fn closed() -> Self {
        Self::Closed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminal {
    pub tab: u64,
    pub pane: Option<u64>,
    pub size: Size,
    pub fit: bool,
}

impl Terminal {
    pub async fn run(self, store: Store, sink: impl FnMut(Screen) + Send) {
        follow::revisit(&store, |session| holds(session, self.tab), self, sink).await;
    }

    fn showing(self, reached: Reached) -> Showing {
        let source = Source::Tab { tab: self.tab, size: self.size };
        Showing { live: Live::open(reached.transport, reached.queue, source), shown: Shown::new(self.pane) }
    }

    async fn fitted<S: FnMut(Screen) + Send>(
        self,
        store: &Store,
        reached: Reached,
        told: &mut Told<Screen, S>,
    ) -> bool {
        let used = reached.transport.clone();
        let claim = Claim::open(used.clone(), self.tab, self.size);
        let mut shown = Shown::new(self.pane);
        let paint = |grid: &Grid, changes: Result<Changes, Error>| match changes {
            Ok(changes) => told.said(shown.update(grid, &changes).map(Screen::Update)),
            Err(error) => told.failed(error),
        };
        let moved_on = follow::moved_on(store, &used);
        let unfitted = match follow_fitted(claim, reached.transport, reached.queue, moved_on, paint).await {
            Fitted::TakenBack => Unfitted::TakenBack,
            Fitted::Refused(error) if follow::still_on(store, &used) => Unfitted::Refused(error),
            Fitted::Ended(ended) => {
                told.ended(ended, follow::still_on(store, &used));
                return true;
            }
            Fitted::MovedAway => {
                left(store, self.tab).await;
                return true;
            }
            Fitted::Refused(_) | Fitted::MovedOn => return true,
        };
        told.say(Screen::Fit(unfitted));
        false
    }
}

impl Visit for Terminal {
    type Said = Screen;

    async fn visit<S: FnMut(Screen) + Send>(
        &mut self,
        store: &Store,
        reached: Reached,
        told: &mut Told<Screen, S>,
    ) -> After {
        if !self.fit {
            let used = reached.transport.clone();
            follow::follow(store, &used, self.showing(reached), told).await;
        } else if !self.fitted(store, reached, told).await {
            self.fit = false;
            return After::AtOnce;
        }
        After::Settling
    }
}

struct Showing {
    live: Live,
    shown: Shown,
}

impl Opened for Showing {
    type Said = Screen;

    async fn next(&mut self) -> follow::Step<Option<Screen>> {
        let step = self.live.next().await;
        step.map(|changes| self.shown.update(self.live.grid(), &changes).map(Screen::Update))
    }
}

async fn left(store: &Store, tab: u64) {
    let mut changes = store.changes();
    let gone = async {
        while store.session().is_some_and(|session| holds(&session, tab)) {
            if changes.next().await.is_none() {
                return;
            }
        }
    };
    let _ = time::timeout(LEAVING, gone).await;
}

pub(crate) fn holds(session: &Snapshot, tab: u64) -> bool {
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

#[cfg(test)]
mod tests;
