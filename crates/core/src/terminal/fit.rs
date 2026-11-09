use std::future::{self, Future};
use std::pin::{Pin, pin};
use std::sync::Arc;

use futures_util::future::{Either, select};

use super::grid::{Changes, Grid};
use super::live::{Ended, Followed, Live, Source, Step};
use crate::cli::surface::Size;
use crate::cli::view::{self, ViewInfo, ViewLine};
use crate::error::Error;
use crate::queue::Queue;
use crate::transport::{Transport, json_error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    Granted { view: u64, projection: u64 },
    Projection(u64),
    TakenBack,
    MovedAway,
    Refused(Error),
    Ended(Ended),
}

pub(crate) struct Claim {
    tab: u64,
    granted: bool,
    followed: Followed,
}

impl Claim {
    pub(crate) fn open(transport: Arc<dyn Transport>, tab: u64, size: Size) -> Self {
        Self { tab, granted: false, followed: Followed::open(transport, view::argv(tab, size)) }
    }

    pub(crate) async fn next(&mut self) -> Verdict {
        match self.followed.next().await {
            Either::Left(line) => self.judged(&line),
            Either::Right(ended) if self.granted || passing(&ended) => Verdict::Ended(ended),
            Either::Right(ended) => Verdict::Refused(refusal(ended)),
        }
    }

    fn judged(&mut self, line: &str) -> Verdict {
        let view = match serde_json::from_str::<ViewLine>(line) {
            Ok(line) => line.view,
            Err(error) => return Verdict::Refused(json_error(&error)),
        };
        if view.shown.tab != Some(self.tab) {
            return Verdict::MovedAway;
        }
        if taken_back(&view) {
            return Verdict::TakenBack;
        }
        if self.granted {
            return Verdict::Projection(view.projection);
        }
        self.granted = true;
        Verdict::Granted { view: view.id, projection: view.projection }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fitted {
    TakenBack,
    MovedAway,
    Refused(Error),
    Ended(Ended),
    MovedOn,
}

enum Heard {
    Verdict(Verdict),
    Step(Step),
    MovedOn,
}

struct Fitting {
    tab: u64,
    transport: Arc<dyn Transport>,
    queue: Queue,
    live: Option<Live>,
}

impl Fitting {
    fn judged(&mut self, verdict: Verdict) -> Option<Fitted> {
        match verdict {
            Verdict::Granted { view, projection } => self.opened(view).saw_projection(projection),
            Verdict::Projection(projection) => self.live.iter_mut().for_each(|live| live.saw_projection(projection)),
            Verdict::TakenBack => return Some(Fitted::TakenBack),
            Verdict::MovedAway => return Some(Fitted::MovedAway),
            Verdict::Refused(error) => return Some(Fitted::Refused(error)),
            Verdict::Ended(ended) => return Some(Fitted::Ended(ended)),
        }
        None
    }

    fn opened(&mut self, view: u64) -> &mut Live {
        let source = Source::View { view, tab: self.tab };
        self.live.get_or_insert_with(|| Live::open(self.transport.clone(), self.queue.clone(), source))
    }
}

pub(crate) async fn follow_fitted(
    mut claim: Claim,
    transport: Arc<dyn Transport>,
    queue: Queue,
    moved_on: impl Future<Output = ()>,
    mut paint: impl FnMut(&Grid, Result<Changes, Error>),
) -> Fitted {
    let mut moved_on = pin!(moved_on);
    let mut fitting = Fitting { tab: claim.tab, transport, queue, live: None };
    loop {
        let over = match heard(&mut claim, fitting.live.as_mut(), moved_on.as_mut()).await {
            Heard::Verdict(verdict) => fitting.judged(verdict),
            Heard::Step(step) => fitting.live.as_ref().and_then(|live| painted(live.grid(), step, &mut paint)),
            Heard::MovedOn => Some(Fitted::MovedOn),
        };
        if let Some(fitted) = over {
            return fitted;
        }
    }
}

async fn heard(claim: &mut Claim, live: Option<&mut Live>, moved_on: Pin<&mut impl Future<Output = ()>>) -> Heard {
    let stepped = async {
        match live {
            Some(live) => live.next().await,
            None => future::pending().await,
        }
    };
    match select(select(pin!(claim.next()), pin!(stepped)), moved_on).await {
        Either::Left((Either::Left((verdict, _)), _)) => Heard::Verdict(verdict),
        Either::Left((Either::Right((step, _)), _)) => Heard::Step(step),
        Either::Right(_) => Heard::MovedOn,
    }
}

fn painted(grid: &Grid, step: Step, paint: &mut impl FnMut(&Grid, Result<Changes, Error>)) -> Option<Fitted> {
    match step {
        Step::Changed(changes) => paint(grid, Ok(changes)),
        Step::Warned(error) => paint(grid, Err(error)),
        Step::Ended(ended) => return Some(Fitted::Ended(ended)),
    }
    None
}

fn taken_back(view: &ViewInfo) -> bool {
    let sized = view.cols.is_some() && view.rows.is_some();
    view.active && sized && !view.controls
}

fn passing(ended: &Ended) -> bool {
    matches!(ended, Ended::Lost(_) | Ended::Failed(Error::StaleBoot(_)))
}

fn refusal(ended: Ended) -> Error {
    match ended {
        Ended::Failed(error) | Ended::Lost(error) => error,
        Ended::Exited => Error::StreamEnded,
    }
}

#[cfg(test)]
mod tests;
