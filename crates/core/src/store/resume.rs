use std::future::Future;
use std::pin::pin;

use futures_util::future::{self, Either};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Resume {
    Foreground,
    Network,
}

impl Resume {
    pub(super) fn interrupts(self) -> bool {
        self == Self::Network
    }
}

pub(super) enum Woken<T> {
    Done(T),
    Resumed,
    Dropped,
}

pub(super) struct Resumes(mpsc::UnboundedReceiver<Resume>);

impl Resumes {
    pub(super) fn new(asked: mpsc::UnboundedReceiver<Resume>) -> Self {
        Self(asked)
    }

    pub(super) fn forget(&mut self) {
        while self.0.try_recv().is_ok() {}
    }

    pub(super) async fn asked(&mut self, now: impl Fn(Resume) -> bool) -> bool {
        while let Some(resume) = self.0.recv().await {
            if now(resume) {
                return true;
            }
        }
        false
    }

    pub(super) async fn race<T>(&mut self, work: impl Future<Output = T>, now: impl Fn(Resume) -> bool) -> Woken<T> {
        match future::select(pin!(work), pin!(self.asked(now))).await {
            Either::Left((done, _)) => Woken::Done(done),
            Either::Right((true, _)) => Woken::Resumed,
            Either::Right((false, _)) => Woken::Dropped,
        }
    }
}
