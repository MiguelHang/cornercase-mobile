use std::future::Future;
use std::pin::pin;

use futures_util::future::{self, Either};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Resume {
    Foreground,
    Network,
    Offline,
    Online,
}

impl Resume {
    pub(super) fn interrupts(self) -> bool {
        matches!(self, Self::Network | Self::Offline)
    }

    fn woken<T>(self) -> Woken<T> {
        if self == Self::Offline { Woken::Offline } else { Woken::Resumed }
    }
}

pub(super) enum Woken<T> {
    Done(T),
    Resumed,
    Offline,
    Dropped,
}

pub(super) struct Resumes {
    asked: mpsc::UnboundedReceiver<Resume>,
    down: bool,
}

impl Resumes {
    pub(super) fn new(asked: mpsc::UnboundedReceiver<Resume>) -> Self {
        Self { asked, down: false }
    }

    pub(super) fn forget(&mut self) -> bool {
        while let Ok(resume) = self.asked.try_recv() {
            self.heard(resume);
        }
        self.down
    }

    pub(super) async fn asked<T>(&mut self, now: impl Fn(Resume) -> bool) -> Woken<T> {
        while let Some(resume) = self.asked.recv().await {
            self.heard(resume);
            if now(resume) {
                return resume.woken();
            }
        }
        Woken::Dropped
    }

    pub(super) async fn race<T>(&mut self, work: impl Future<Output = T>, now: impl Fn(Resume) -> bool) -> Woken<T> {
        match future::select(pin!(work), pin!(self.asked(now))).await {
            Either::Left((done, _)) => Woken::Done(done),
            Either::Right((woken, _)) => woken,
        }
    }

    fn heard(&mut self, resume: Resume) {
        match resume {
            Resume::Offline => self.down = true,
            Resume::Online => self.down = false,
            Resume::Foreground | Resume::Network => {}
        }
    }
}
