use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;

use super::grid::{Applied, Changes, Grid};
use super::shown::{Shown, Update};
use crate::cli::status::Snapshot;
use crate::cli::surface::{Frame, Size};
use crate::error::Error;
use crate::follow::{self, Lines, Notice, Opened, Reached};
use crate::queue::Queue;
use crate::store::Store;
use crate::transport::{Transport, json_error};

const REDRAW: &str = "redraw";
const REDRAW_AGAIN: Duration = Duration::from_secs(2);

pub type Step = follow::Step<Changes>;

pub struct Live {
    lines: Lines,
    drawing: Drawing,
}

struct Drawing {
    queue: Queue,
    grid: Grid,
    redrawing: Option<Instant>,
}

impl Live {
    #[must_use]
    pub fn open(transport: Arc<dyn Transport>, queue: Queue, tab: u64, size: Size) -> Self {
        let lines = Lines::open(transport, surface(tab, size));
        Self { lines, drawing: Drawing { queue, grid: Grid::default(), redrawing: None } }
    }

    #[must_use]
    pub fn grid(&self) -> &Grid {
        &self.drawing.grid
    }

    pub async fn next(&mut self) -> Step {
        self.lines.next(|line| self.drawing.applied(line)).await
    }
}

impl Drawing {
    fn applied(&mut self, line: &str) -> Option<Step> {
        let frame: Frame = match serde_json::from_str(line) {
            Ok(frame) => frame,
            Err(error) => return Some(Step::Warned(json_error(&error))),
        };
        let surface = frame.surface;
        if frame.base.is_some() && self.still_redrawing(surface) {
            return None;
        }
        if self.grid.apply(frame) == Applied::NeedRedraw {
            self.redraw(surface);
            return None;
        }
        self.redrawing = None;
        Some(Step::Changed(self.grid.changed_rows()))
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
}

impl Terminal {
    pub async fn run(self, store: Store, sink: impl FnMut(Screen) + Send) {
        let showing = |reached: Reached| Showing {
            live: Live::open(reached.transport, reached.queue, self.tab, self.size),
            shown: Shown::new(self.pane),
        };
        follow::reopen(&store, |session| holds(session, self.tab), showing, sink).await;
    }
}

struct Showing {
    live: Live,
    shown: Shown,
}

impl Opened for Showing {
    type Said = Screen;

    async fn next(&mut self) -> follow::Step<Option<Screen>> {
        match self.live.next().await {
            Step::Changed(changes) => {
                follow::Step::Changed(self.shown.update(self.live.grid(), &changes).map(Screen::Update))
            }
            Step::Warned(error) => follow::Step::Warned(error),
            Step::Ended(ended) => follow::Step::Ended(ended),
        }
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

#[cfg(test)]
mod tests;
