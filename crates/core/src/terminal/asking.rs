use super::grid::{Grid, Line};
use super::live::{Live, Source, holds};
use super::question::{Choice, cropped, options};
use super::shown::Shown;
use crate::cli::surface::{Colours, Size};
use crate::error::Error;
use crate::follow::{self, Notice, Opened, Reached, Step};
use crate::store::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub pane: u64,
    pub colours: Colours,
    pub rows: Vec<Line>,
    pub options: Vec<Choice>,
}

impl Question {
    fn of(shown: &Shown, grid: &Grid) -> Option<Self> {
        let pane = shown.chosen(grid)?;
        let rows = cropped(pane);
        Some(Self { pane: pane.pane, colours: pane.colours.clone(), rows: rows.to_vec(), options: options(rows) })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Question(Question),
    Failed(Error),
    Closed,
}

impl Notice for Asked {
    fn failed(error: Error) -> Self {
        Self::Failed(error)
    }

    fn closed() -> Self {
        Self::Closed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asking {
    pub tab: u64,
    pub pane: u64,
    pub size: Size,
}

impl Asking {
    pub async fn run(self, store: Store, sink: impl FnMut(Asked) + Send) {
        let reading = |reached: Reached| Reading {
            live: Live::open(reached.transport, reached.queue, Source::Tab { tab: self.tab, size: self.size }),
            shown: Shown::new(Some(self.pane)),
            last: None,
        };
        follow::reopen(&store, |session| holds(session, self.tab), reading, sink).await;
    }
}

struct Reading {
    live: Live,
    shown: Shown,
    last: Option<Question>,
}

impl Reading {
    fn asked(&mut self) -> Option<Asked> {
        let question = Question::of(&self.shown, self.live.grid())?;
        if self.last.as_ref() == Some(&question) {
            return None;
        }
        self.last = Some(question.clone());
        Some(Asked::Question(question))
    }
}

impl Opened for Reading {
    type Said = Asked;

    async fn next(&mut self) -> Step<Option<Asked>> {
        let step = self.live.next().await;
        step.map(|_| self.asked())
    }
}

#[cfg(test)]
mod tests;
