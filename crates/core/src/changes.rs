use std::sync::atomic::{AtomicBool, Ordering};

use crate::cli::changes::{Mode, State, Summary, follow_args};
use crate::cli::status::Snapshot;
use crate::error::Error;
use crate::follow::{self, Lines, Notice, Opened, Reached, Step};
use crate::store::Store;
use crate::transport::json_error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    Summary(Summary),
    Failed(Error),
    Closed,
}

impl Notice for Seen {
    fn failed(error: Error) -> Self {
        Self::Failed(error)
    }

    fn closed() -> Self {
        Self::Closed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Changes {
    pub workspace: u64,
    pub mode: Mode,
}

impl Changes {
    pub async fn run(self, store: Store, sink: impl FnMut(Seen) + Send) {
        let listed = AtomicBool::new(false);
        let following = |reached: Reached| Following {
            lines: Lines::open(reached.transport, follow_args(self.workspace, self.mode)),
            listed: &listed,
        };
        follow::reopen(&store, |session| lists(session, self.workspace), following, sink).await;
    }
}

struct Following<'a> {
    lines: Lines,
    listed: &'a AtomicBool,
}

impl Opened for Following<'_> {
    type Said = Seen;

    async fn next(&mut self) -> Step<Option<Seen>> {
        let listed = self.listed;
        self.lines.next(|line| read(line, listed)).await
    }
}

fn read(line: &str, listed: &AtomicBool) -> Option<Step<Option<Seen>>> {
    let summary: Summary = match serde_json::from_str(line) {
        Ok(summary) => summary,
        Err(error) => return Some(Step::Warned(json_error(&error))),
    };
    if listed.swap(true, Ordering::Relaxed) && only_starting(&summary) {
        return None;
    }
    Some(Step::Changed(Some(Seen::Summary(summary))))
}

fn only_starting(summary: &Summary) -> bool {
    summary.state == State::Loading && summary.workspaces.iter().all(|workspace| workspace.files.is_empty())
}

fn lists(session: &Snapshot, workspace: u64) -> bool {
    session.projects.iter().flat_map(|project| &project.workspaces).any(|open| open.id == workspace)
}

#[cfg(test)]
mod tests;
