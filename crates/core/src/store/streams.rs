use std::pin::pin;

use futures_util::future::{self, BoxFuture, Either};
use futures_util::{FutureExt, StreamExt, stream};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc;

use super::{Change, Connection, Shared};
use crate::cli::events::Event;
use crate::cli::status::Snapshot;
use crate::error::Error;
use crate::machines::pick::Picked;
use crate::machines::route::Address;
use crate::transport::{Exit, Output, Transport, json_error, listen, only_stdout, printed};

pub(super) const EVENTS: &[&str] = &["events", "--json", "--notices"];
pub(super) const STATUS: &[&str] = &["status", "--follow", "--json"];
pub(super) const APP: &[&str] = &["status", "--app", "--follow", "--json"];

type End = Result<Exit, Error>;

struct Opened<'a> {
    lines: mpsc::UnboundedReceiver<String>,
    run: BoxFuture<'a, End>,
}

pub(super) async fn follow(shared: &Shared, picked: &Picked) -> Error {
    let transport = picked.transport.as_ref();
    let route = picked.route.address();
    let streams = [
        pump(open(transport, EVENTS), |line| shared.event(line)).boxed(),
        pump(open(transport, STATUS), |line| shared.status(line, route)).boxed(),
        pump(open(transport, APP), |line| shared.app(line)).boxed(),
    ];
    let (end, _, _) = future::select_all(streams).await;
    ended(end)
}

fn open<'a>(transport: &'a dyn Transport, argv: &[&str]) -> Opened<'a> {
    let (out, lines) = listen(only_stdout);
    let argv = argv.iter().map(|&word| word.to_owned()).collect();
    Opened { lines, run: transport.run(argv, stream::pending().boxed(), out) }
}

async fn pump(mut opened: Opened<'_>, apply: impl Fn(&str)) -> End {
    loop {
        match heard(&mut opened).await {
            Either::Left(line) => apply(&line),
            Either::Right(end) => return drained(&mut opened.lines, &apply, end),
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

fn drained(lines: &mut mpsc::UnboundedReceiver<String>, apply: &impl Fn(&str), end: End) -> End {
    while let Ok(line) = lines.try_recv() {
        apply(&line);
    }
    end
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
            self.publish(&Change::Event(Box::new(event)));
        }
    }

    fn status(&self, line: &str, route: &Address) {
        let Some(snapshot) = self.parsed::<Snapshot>(line) else {
            return;
        };
        self.snapshot(snapshot);
        if !self.connected() {
            self.set_connection(Connection::Connected { route: route.clone() });
        }
    }

    fn snapshot(&self, snapshot: Snapshot) {
        let mut state = self.state();
        let last = state.session.as_ref();
        if last.is_some_and(|last| last.boot == snapshot.boot && snapshot.revision <= last.revision) {
            return;
        }
        state.session = Some(snapshot);
        drop(state);
        self.publish(&Change::Session);
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
