use std::future::{self, Future};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::SinkExt;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, Instant};

use super::End;
use crate::cli::contract::fixtures::REAL;
use crate::cli::status::fixtures::WATCH;
use crate::error::Error;
use crate::machines::pick::PickConfig;
use crate::machines::route::{Address, Route, Token};
use crate::machines::{Machine, MachineId};
use crate::store::Store;
use crate::transport::fake::FakeTransport;
use crate::transport::{Exit, Out, OutSink};

pub(crate) const BOOT: &str = "4242-1760600000";
pub(crate) const RESTARTED: &str = "4243-1760600100";

#[derive(Clone)]
pub(crate) enum Said {
    Line(String),
    After(u64),
    Ends(End),
}

pub(crate) fn exits() -> Said {
    Said::Ends(Ok(Exit { code: 0, stderr: String::new() }))
}

pub(crate) async fn played(mut out: OutSink, said: Vec<Said>) -> End {
    for said in said {
        match said {
            Said::Line(line) => out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)?,
            Said::After(seconds) => time::sleep(Duration::from_secs(seconds)).await,
            Said::Ends(end) => return end,
        }
    }
    future::pending().await
}

pub(crate) fn contract(boot: &str) -> Vec<Said> {
    let mut contract: Value = serde_json::from_str(REAL).expect("the real contract");
    contract["boot"] = boot.into();
    vec![Said::Line(contract.to_string()), exits()]
}

pub(crate) fn session(boot: &str, revision: u64, with_projects: bool) -> Said {
    let mut session: Value = serde_json::from_str(WATCH).expect("the watch fixture");
    (session["boot"], session["revision"]) = (boot.into(), revision.into());
    if !with_projects {
        session["projects"] = Value::Array(Vec::new());
    }
    Said::Line(session.to_string())
}

pub(crate) fn connected() -> Vec<Vec<Said>> {
    vec![vec![session(BOOT, 7, true)]]
}

pub(crate) fn dropped() -> Said {
    Said::Ends(Err(Error::Dropped))
}

pub(crate) fn dropped_for_a_while() -> Vec<Vec<Said>> {
    vec![vec![session(BOOT, 7, true), Said::After(5), dropped()], vec![Said::After(20), session(BOOT, 7, true)]]
}

pub(crate) fn refused(stderr: &str) -> Said {
    Said::Ends(Ok(Exit { code: 1, stderr: stderr.to_owned() }))
}

fn in_turn(fake: FakeTransport, argv: &[&str], turns: Vec<Vec<Said>>) -> FakeTransport {
    let call = AtomicUsize::new(0);
    fake.script(argv, move |_, out| {
        let turn = turns.get(call.fetch_add(1, Ordering::SeqCst)).or(turns.last());
        played(out, turn.cloned().unwrap_or_default())
    })
}

pub(crate) struct Mac {
    pub(crate) contracts: Vec<Vec<Said>>,
    pub(crate) sessions: Vec<Vec<Said>>,
    pub(crate) streams: Vec<Vec<Said>>,
    pub(crate) followed: &'static [&'static str],
    pub(crate) also: Vec<(&'static [&'static str], Vec<Vec<Said>>)>,
}

impl Mac {
    pub(crate) fn of(followed: &'static [&'static str], sessions: Vec<Vec<Said>>, streams: Vec<Vec<Said>>) -> Self {
        Self { contracts: vec![contract(BOOT)], sessions, streams, followed, also: Vec::new() }
    }

    pub(crate) fn also(mut self, argv: &'static [&'static str], turns: Vec<Vec<Said>>) -> Self {
        self.also.push((argv, turns));
        self
    }

    pub(crate) fn restarting(followed: &'static [&'static str], streams: Vec<Vec<Said>>) -> Self {
        let restart = vec![session(BOOT, 7, true), Said::After(5), session(RESTARTED, 1, true)];
        let sessions = vec![restart, vec![Said::After(10), session(RESTARTED, 1, true)]];
        Self { contracts: vec![contract(BOOT), contract(RESTARTED)], ..Self::of(followed, sessions, streams) }
    }

    fn fake(self) -> FakeTransport {
        let fake = FakeTransport::default().open(&["events", "--json", "--notices"], &[]);
        let fake = in_turn(fake.open(&["status", "--app", "--follow", "--json"], &[]), self.followed, self.streams);
        let fake = self.also.into_iter().fold(fake, |fake, (argv, turns)| in_turn(fake, argv, turns));
        let fake = in_turn(fake, &["contract", "--json"], self.contracts);
        in_turn(fake, &["status", "--follow", "--json"], self.sessions)
    }
}

pub(crate) type Sink<T> = Box<dyn FnMut(T) + Send>;

pub(crate) struct Watched<T> {
    said: mpsc::UnboundedReceiver<(Duration, T)>,
    launched: mpsc::UnboundedReceiver<String>,
    pub(crate) running: JoinHandle<()>,
}

impl<T: Send + 'static> Watched<T> {
    pub(crate) async fn after_a_minute<F>(mac: Mac, run: impl FnOnce(Store, Sink<T>) -> F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (transport, launched) = mac.fake().launched();
        let address = Address { host: "studio.local".to_owned(), port: 8765 };
        let studio = Route::cli(address, Some(Token::new("4f2a91c0.9c1e2a7b5d0f".to_owned())));
        let machine = Machine { id: MachineId(1), name: "studio".to_owned(), routes: vec![studio], last_route: None };
        let (store, driver) = Store::new(machine, move |_: &Route| Ok(transport.clone()), PickConfig::default());
        tokio::spawn(driver);
        let (sender, said) = mpsc::unbounded_channel();
        let started = Instant::now();
        let running = tokio::spawn(run(
            store,
            Box::new(move |told| sender.send((started.elapsed(), told)).expect("the test reads all that is said")),
        ));
        time::sleep(Duration::from_secs(60)).await;
        Self { said, launched, running }
    }

    pub(crate) fn seen<S>(&mut self, what: impl Fn(T) -> S) -> Vec<(u64, S)> {
        let mut seen = Vec::new();
        while let Ok((at, told)) = self.said.try_recv() {
            seen.push((at.as_secs(), what(told)));
        }
        seen
    }

    pub(crate) fn launched(&mut self, commands: &[&str]) -> Vec<String> {
        let mut launched = Vec::new();
        while let Ok(argv) = self.launched.try_recv() {
            launched.extend(commands.iter().any(|command| argv.starts_with(command)).then_some(argv));
        }
        launched
    }
}
