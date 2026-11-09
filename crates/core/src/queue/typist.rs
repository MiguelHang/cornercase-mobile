use std::mem;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::Either;
use serde_json::Value;
use tokio::time;

use super::follow::{End, Follow, Gone};
use crate::error::Error;
use crate::transport::{Exit, Transport, json_error};

const FOLLOW: [&str; 4] = ["input", "--follow", "--json", "-"];
const SETTLE: Duration = Duration::from_secs(10);
const UNANSWERED: usize = 256;

pub(crate) type OnError = Arc<dyn Fn(Error) + Send + Sync>;

pub(crate) type Event = (u64, Value);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Typed {
    Done,
    Gone,
    Unsent(Vec<Event>),
    Absent(Vec<Event>),
}

pub(crate) struct Typist {
    follow: Follow,
    on_error: OnError,
    unanswered: usize,
    unconfirmed: Vec<Event>,
}

impl Typist {
    pub(crate) fn start(transport: Arc<dyn Transport>, on_error: OnError) -> Self {
        Self { follow: Follow::start(transport, &FOLLOW), on_error, unanswered: 0, unconfirmed: Vec::new() }
    }

    pub(crate) async fn send(&mut self, pane: u64, event: &Value) -> Typed {
        self.read_ready();
        if self.unanswered >= UNANSWERED {
            let settled = self.settle().await;
            if settled != Typed::Done {
                return settled;
            }
        }
        match self.follow.write(&line(pane, event)).await {
            Ok(()) => {
                self.unanswered += 1;
                self.unconfirmed.push((pane, event.clone()));
                Typed::Done
            }
            Err(None) => Typed::Gone,
            Err(Some(end)) => self.gone(&end),
        }
    }

    pub(crate) async fn settle(&mut self) -> Typed {
        if let Ok(typed) = time::timeout(SETTLE, self.answered()).await {
            return typed;
        }
        (self.on_error)(Error::SettleTimeout);
        Typed::Gone
    }

    pub(crate) async fn ended(&mut self) -> Typed {
        loop {
            if let Some(gone) = self.hear().await {
                return gone;
            }
        }
    }

    async fn answered(&mut self) -> Typed {
        while self.unanswered > 0 {
            if let Some(gone) = self.hear().await {
                return gone;
            }
        }
        Typed::Done
    }

    async fn hear(&mut self) -> Option<Typed> {
        if self.follow.is_over() {
            return Some(Typed::Gone);
        }
        match self.follow.next().await {
            Either::Left(line) => {
                self.read(&line);
                None
            }
            Either::Right(end) => Some(self.gone(&end)),
        }
    }

    fn read(&mut self, line: &str) {
        let (events, rejected) = answer(line);
        self.unanswered = self.unanswered.saturating_sub(events);
        self.unconfirmed.clear();
        if let Some(error) = rejected {
            (self.on_error)(error);
        }
    }

    fn read_ready(&mut self) {
        while let Some(line) = self.follow.ready() {
            self.read(&line);
        }
    }

    fn gone(&mut self, end: &End) -> Typed {
        self.read_ready();
        match (self.follow.gone(end), end) {
            (Gone::Absent, _) => Typed::Absent(mem::take(&mut self.unconfirmed)),
            (Gone::Unopened, _) => Typed::Unsent(mem::take(&mut self.unconfirmed)),
            (Gone::Lost, Ok(exit)) => self.warn(stopped(exit)),
            (Gone::Lost, Err(error)) => self.warn(error.clone()),
        }
    }

    fn warn(&self, error: Error) -> Typed {
        (self.on_error)(error);
        Typed::Gone
    }
}

pub(crate) fn line(pane: u64, event: &Value) -> String {
    let mut event = event.clone();
    if let Some(fields) = event.as_object_mut() {
        fields.insert("pane".to_owned(), pane.into());
    }
    event.to_string()
}

fn answer(line: &str) -> (usize, Option<Error>) {
    let answer = match serde_json::from_str::<Value>(line) {
        Ok(answer) => answer,
        Err(error) => return (1, Some(json_error(&error))),
    };
    let events = answer["events"].as_u64().and_then(|events| usize::try_from(events).ok()).unwrap_or(1);
    (events, answer["error"].as_str().map(|error| Error::InputRejected(error.to_owned())))
}

fn stopped(exit: &Exit) -> Error {
    let why = exit.stderr.lines().map(str::trim).rfind(|line| !line.is_empty());
    let why = why.unwrap_or("`cornercase input --follow` ended");
    Error::Failed { code: exit.code, stderr: why.to_owned() }
}

#[cfg(test)]
mod tests;
