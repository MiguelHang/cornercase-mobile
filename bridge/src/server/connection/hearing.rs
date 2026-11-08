use std::fmt;
use std::future::pending;
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tokio::time::{Instant, sleep_until};
use tokio_tungstenite::tungstenite::Error;

use cornercase_core::protocol::Refusal;

use crate::server::{Stop, stopped};

#[derive(Debug)]
pub(super) enum Ending {
    Finished,
    Refused(Refusal),
    Closed,
    Broken(Error),
    Silent(Duration),
    Stopped,
}

impl fmt::Display for Ending {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Finished => formatter.write_str("the command ended"),
            Self::Refused(_) => formatter.write_str("the client sent a bad message"),
            Self::Closed => formatter.write_str("the client closed it"),
            Self::Broken(error) => write!(formatter, "the connection broke: {error}"),
            Self::Silent(silence) => write!(formatter, "the client was silent for {silence:?}"),
            Self::Stopped => formatter.write_str("the bridge is stopping"),
        }
    }
}

pub(super) struct Ears {
    heard: watch::Sender<Option<Instant>>,
    ended: mpsc::Sender<Ending>,
}

pub(super) struct Hearing {
    heard: watch::Receiver<Option<Instant>>,
    endings: mpsc::Receiver<Ending>,
    silence: Duration,
    stop: Stop,
}

pub(super) fn listen(silence: Duration, stop: Stop) -> (Ears, Hearing) {
    let (heard, last) = watch::channel(Some(Instant::now()));
    let (ended, endings) = mpsc::channel(1);
    (Ears { heard, ended }, Hearing { heard: last, endings, silence, stop })
}

impl Ears {
    pub(super) fn heard(&self) {
        self.heard.send_replace(Some(Instant::now()));
    }

    pub(super) fn busy(&self) {
        self.heard.send_replace(None);
    }

    pub(super) fn end(&self, ending: Ending) {
        let _ = self.ended.try_send(ending);
    }
}

impl Hearing {
    pub(super) async fn ending(&mut self) -> Ending {
        let silence = self.silence;
        tokio::select! {
            biased;
            Some(ending) = self.endings.recv() => ending,
            () = stopped(&mut self.stop) => Ending::Stopped,
            () = silent(&mut self.heard, silence) => Ending::Silent(silence),
        }
    }
}

async fn silent(heard: &mut watch::Receiver<Option<Instant>>, silence: Duration) {
    loop {
        let last = *heard.borrow_and_update();
        tokio::select! {
            () = quiet(last, silence) => return,
            changed = heard.changed() => if changed.is_err() {
                return;
            },
        }
    }
}

async fn quiet(last: Option<Instant>, silence: Duration) {
    match last {
        Some(at) => sleep_until(at + silence).await,
        None => pending().await,
    }
}

#[cfg(test)]
mod tests;
