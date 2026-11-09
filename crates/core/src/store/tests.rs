use std::iter;
use std::sync::PoisonError;
use std::time::Duration;

use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;
use tokio::time::{self, Instant};

use super::*;
use crate::machines::MachineId;
use crate::transport::fake::FakeTransport;
use crate::transport::{Exit, OutSink, Stdin};

mod connect;
mod retry;
mod streams;

const CONTRACT: &[&str] = &["contract", "--json"];
const LAN: &str = "studio.local";
const TAILNET: &str = "studio.tailnet.ts.net";
const TOKEN: &str = "c2VjcmV0LXRva2Vu";

#[derive(Clone, Default)]
struct Asked(Arc<Mutex<Vec<String>>>);

impl Asked {
    fn record(&self, host: &str, argv: &[String]) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).push(format!("{host}: {}", argv.join(" ")));
    }

    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

struct Recording {
    host: &'static str,
    fake: FakeTransport,
    asked: Asked,
}

impl Transport for Recording {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>> {
        self.asked.record(self.host, &argv);
        self.fake.run(argv, stdin, out)
    }
}

fn route(host: &str) -> Route {
    Route::parse(&format!("ws://{host}:8765/?token={TOKEN}")).expect("a bridge url")
}

fn machine(routes: Vec<Route>) -> Machine {
    Machine { id: MachineId(1), name: "studio".to_owned(), routes, last_route: None }
}

fn network(
    hosts: Vec<(&'static str, FakeTransport)>,
    asked: &Asked,
) -> impl Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync + 'static + use<> {
    let transports: Vec<(&str, Arc<dyn Transport>)> = hosts
        .into_iter()
        .map(|(host, fake)| (host, Arc::new(Recording { host, fake, asked: asked.clone() }) as Arc<dyn Transport>))
        .collect();
    move |route: &Route| {
        let found = transports.iter().find(|(host, _)| *host == route.address().host);
        found.map(|(_, transport)| transport.clone()).ok_or_else(|| Error::Connect("no route to host".to_owned()))
    }
}

fn store_on(
    routes: Vec<Route>,
    hosts: Vec<(&'static str, FakeTransport)>,
    asked: &Asked,
) -> (Store, impl Future<Output = ()> + Send + 'static + use<>) {
    Store::new(machine(routes), network(hosts, asked), PickConfig::default())
}

fn unreachable() -> (Store, impl Future<Output = ()> + Send + 'static) {
    store_on(vec![route(LAN)], Vec::new(), &Asked::default())
}

async fn settled(store: &Store, driver: impl Future<Output = ()> + Send + 'static) -> Vec<Change> {
    let mut changes = store.changes();
    tokio::spawn(driver);
    time::sleep(Duration::from_secs(60)).await;
    unread(&mut changes)
}

fn unread(changes: &mut Changes) -> Vec<Change> {
    iter::from_fn(|| changes.next().now_or_never().flatten()).collect()
}

async fn next_connection(changes: &mut Changes) -> Connection {
    loop {
        if let Some(Change::Connection(connection)) = changes.next().await {
            return connection;
        }
    }
}

async fn retrying(changes: &mut Changes) -> (Duration, Error) {
    loop {
        if let Connection::Retrying { r#in, error } = next_connection(changes).await {
            return (r#in, error);
        }
    }
}

async fn next_wait(changes: &mut Changes) -> Duration {
    retrying(changes).await.0
}

async fn drain(changes: &mut Changes) {
    while changes.next().await.is_some() {}
}

mod the_driver {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn waits_longer_after_each_failed_attempt() {
        let (store, driver) = unreachable();
        let mut changes = store.changes();
        let started = Instant::now();
        tokio::spawn(driver);
        let waits = [next_wait(&mut changes).await, next_wait(&mut changes).await, next_wait(&mut changes).await];
        assert_eq!(waits, [1, 2, 4].map(Duration::from_secs));
        assert_eq!(started.elapsed(), Duration::from_secs(3));
    }

    #[tokio::test(start_paused = true)]
    async fn ends_once_the_last_handle_is_dropped() {
        let (store, driver) = unreachable();
        let copy = store.clone();
        let driver = tokio::spawn(driver);
        drop(store);
        time::sleep(Duration::from_secs(30)).await;
        assert!(!driver.is_finished());
        drop(copy);
        let ended = time::timeout(Duration::from_secs(1), driver).await;
        ended.expect("the driver ends").expect("the driver does not panic");
    }

    #[tokio::test(start_paused = true)]
    async fn ends_the_changes_with_it() {
        let (store, driver) = unreachable();
        let mut changes = store.changes();
        drop(store);
        driver.await;
        time::timeout(Duration::from_secs(1), drain(&mut changes)).await.expect("the changes end");
    }
}

mod the_handle {
    use super::*;

    #[test]
    fn starts_connecting_with_nothing_known() {
        let (store, _driver) = unreachable();
        assert_eq!(store.connection(), Connection::Connecting);
        assert_eq!(store.session(), None);
        assert_eq!(store.app(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn shows_the_connection_it_announced() {
        let (store, driver) = unreachable();
        let mut changes = store.changes();
        tokio::spawn(driver);
        let wait = next_wait(&mut changes).await;
        assert_eq!(wait, Duration::from_secs(1));
        assert!(matches!(store.connection(), Connection::Retrying { r#in, .. } if r#in == wait));
    }
}

mod changes {
    use super::*;

    #[test]
    fn coalesce_a_session_nobody_read_yet() {
        let (store, _driver) = unreachable();
        let mut changes = store.changes();
        store.shared.publish(&Change::Session);
        store.shared.publish(&Change::App);
        store.shared.publish(&Change::Session);
        store.shared.publish(&Change::App);
        assert_eq!(unread(&mut changes), [Change::Session, Change::App]);
    }

    #[test]
    fn send_a_session_again_once_the_last_one_was_read() {
        let (store, _driver) = unreachable();
        let mut changes = store.changes();
        store.shared.publish(&Change::Session);
        assert_eq!(unread(&mut changes), [Change::Session]);
        store.shared.publish(&Change::Session);
        assert_eq!(unread(&mut changes), [Change::Session]);
    }

    #[test]
    fn keep_every_warning_in_order() {
        let (store, _driver) = unreachable();
        let mut changes = store.changes();
        store.shared.publish(&Change::Warning(Error::Timeout));
        store.shared.publish(&Change::Warning(Error::Dropped));
        store.shared.publish(&Change::Warning(Error::Timeout));
        let warnings = [Error::Timeout, Error::Dropped, Error::Timeout].map(Change::Warning);
        assert_eq!(unread(&mut changes), warnings);
    }

    #[test]
    fn reach_every_subscriber_on_its_own() {
        let (store, _driver) = unreachable();
        let mut first = store.changes();
        store.shared.publish(&Change::Session);
        assert_eq!(unread(&mut first), [Change::Session]);
        let mut second = store.changes();
        store.shared.publish(&Change::Session);
        store.shared.publish(&Change::Session);
        assert_eq!(unread(&mut second), [Change::Session]);
        assert_eq!(unread(&mut first), [Change::Session]);
    }

    #[test]
    fn forget_a_subscriber_that_stopped_listening() {
        let (store, _driver) = unreachable();
        drop(store.changes());
        let _listening = store.changes();
        store.shared.publish(&Change::App);
        assert_eq!(store.shared.state().subscribers.len(), 1);
    }
}
