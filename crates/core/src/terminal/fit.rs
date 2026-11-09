use std::sync::Arc;

use futures_util::future::Either;

use super::live::{Ended, Followed};
use crate::cli::surface::Size;
use crate::cli::view::{self, ViewInfo, ViewLine};
use crate::error::Error;
use crate::transport::{Transport, json_error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Granted { view: u64, projection: u64 },
    Projection(u64),
    TakenBack,
    MovedAway,
    Refused(Error),
}

pub struct Claim {
    tab: u64,
    granted: bool,
    followed: Followed,
}

impl Claim {
    #[must_use]
    pub fn open(transport: Arc<dyn Transport>, tab: u64, size: Size) -> Self {
        Self { tab, granted: false, followed: Followed::open(transport, view::argv(tab, size)) }
    }

    pub async fn next(&mut self) -> Verdict {
        match self.followed.next().await {
            Either::Left(line) => self.judged(&line),
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

fn taken_back(view: &ViewInfo) -> bool {
    let sized = view.cols.is_some() && view.rows.is_some();
    view.active && sized && !view.controls
}

fn refusal(ended: Ended) -> Error {
    match ended {
        Ended::Failed(error) | Ended::Lost(error) => error,
        Ended::Exited => Error::StreamEnded,
    }
}

#[cfg(test)]
mod tests;
