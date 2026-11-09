use std::ops::ControlFlow::{self, Break, Continue};
use std::pin::pin;

use futures_util::future::{self, BoxFuture, Either};
use futures_util::{FutureExt, StreamExt, stream};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc;

use super::{Change, Connection, Shared};
use crate::cli::contract::boot::Boot;
use crate::cli::events::{Event, What};
use crate::cli::status::Snapshot;
use crate::error::Error;
use crate::machines::pick::Picked;
use crate::machines::route::Address;
use crate::transport::{Exit, Output, Transport, json_error, listen, only_stdout, printed};

pub(super) const EVENTS: &[&str] = &["events", "--json", "--notices"];
pub(super) const STATUS: &[&str] = &["status", "--follow", "--json"];
pub(super) const APP: &[&str] = &["status", "--app", "--follow", "--json"];
const UNBOOTED: &str = "not the expected shape: a snapshot without its boot";

type End = Result<Exit, Error>;

pub(super) enum Ended {
    Failed(Error),
    Restarted,
}

struct Opened<'a> {
    lines: mpsc::UnboundedReceiver<String>,
    run: BoxFuture<'a, End>,
}

pub(super) async fn follow(shared: &Shared, picked: &Picked) -> Ended {
    let transport = picked.transport.as_ref();
    let route = picked.route.address();
    shared.doubt();
    let streams = [
        pump(open(transport, EVENTS), going(|line| shared.event(line))).boxed(),
        pump(open(transport, STATUS), |line| shared.status(line, route)).boxed(),
        pump(open(transport, APP), going(|line| shared.app(line))).boxed(),
    ];
    let (ended, _, _) = future::select_all(streams).await;
    ended
}

fn going(apply: impl Fn(&str)) -> impl Fn(&str) -> ControlFlow<()> {
    move |line| {
        apply(line);
        Continue(())
    }
}

fn open<'a>(transport: &'a dyn Transport, argv: &[&str]) -> Opened<'a> {
    let (out, lines) = listen(only_stdout);
    let argv = argv.iter().map(|&word| word.to_owned()).collect();
    Opened { lines, run: transport.run(argv, stream::pending().boxed(), out) }
}

async fn pump(mut opened: Opened<'_>, apply: impl Fn(&str) -> ControlFlow<()>) -> Ended {
    loop {
        let flow = match heard(&mut opened).await {
            Either::Left(line) => apply(&line),
            Either::Right(end) => return drained(&mut opened.lines, &apply, end),
        };
        if flow.is_break() {
            return Ended::Restarted;
        }
    }
}

async fn heard(opened: &mut Opened<'_>) -> Either<String, End> {
    match future::select(pin!(opened.lines.recv()), &mut opened.run).await {
        Either::Left((Some(line), _)) => Either::Left(line),
        Either::Left((None, run)) => Either::Right(run.await),
        Either::Right((end, _)) => Either::Right(end),
    }
}

fn drained(lines: &mut mpsc::UnboundedReceiver<String>, apply: &impl Fn(&str) -> ControlFlow<()>, end: End) -> Ended {
    while let Ok(line) = lines.try_recv() {
        if apply(&line).is_break() {
            return Ended::Restarted;
        }
    }
    Ended::Failed(ended(end))
}

fn ended(end: End) -> Error {
    match end {
        Ok(exit) => printed(Output { stdout: String::new(), stderr: exit.stderr, code: exit.code })
            .err()
            .unwrap_or(Error::StreamEnded),
        Err(error) => error,
    }
}

impl Shared {
    fn event(&self, line: &str) {
        if let Some(event) = self.parsed::<Event>(line) {
            self.heard(&event);
            self.publish(&Change::Event(Box::new(event)));
        }
    }

    fn heard(&self, event: &Event) {
        match (&event.what, event.pane) {
            (What::Status { to, .. }, Some(pane)) => self.state().since.heard(pane, to.as_deref(), &event.time),
            (What::Dropped { .. }, _) => {
                self.state().since.doubt();
            }
            _ => {}
        }
    }

    fn doubt(&self) {
        let doubted = self.state().since.doubt();
        if doubted {
            self.publish(&Change::Session);
        }
    }

    fn status(&self, line: &str, route: &Address) -> ControlFlow<()> {
        let Some(snapshot) = self.parsed::<Snapshot>(line) else {
            return Continue(());
        };
        if snapshot.boot.is_empty() {
            self.publish(&Change::Warning(Error::Json(UNBOOTED.to_owned())));
            return Continue(());
        }
        self.snapshot(snapshot)?;
        if !self.connected() {
            self.set_connection(Connection::Connected { route: route.clone() });
        }
        Continue(())
    }

    fn snapshot(&self, snapshot: Snapshot) -> ControlFlow<()> {
        if self.restarted(Boot(snapshot.boot.clone())) {
            return Break(());
        }
        let mut state = self.state();
        if state.session.as_ref().is_some_and(|last| snapshot.revision <= last.revision) {
            return Continue(());
        }
        state.since.reconcile(&snapshot);
        state.session = Some(snapshot);
        drop(state);
        self.publish(&Change::Session);
        Continue(())
    }

    fn app(&self, line: &str) {
        if let Some(app) = self.parsed::<Value>(line) {
            self.state().app = Some(app);
            self.publish(&Change::App);
        }
    }

    fn parsed<T: DeserializeOwned>(&self, line: &str) -> Option<T> {
        let parsed = serde_json::from_str(line);
        parsed.inspect_err(|error| self.publish(&Change::Warning(json_error(error)))).ok()
    }
}
