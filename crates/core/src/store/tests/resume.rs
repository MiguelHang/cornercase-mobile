use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::SinkExt;
use rstest::rstest;

use super::boot::late;
use super::connect::{contract, refusing, without_notices};
use super::retry::{exit, held};
use super::streams::{connected, connections, fine, line, served, serving, status_event, watched};
use super::*;
use crate::cli::contract::Kind;
use crate::cli::status::fixtures::WATCH;
use crate::store::streams::{APP, EVENTS, STATUS};
use crate::transport::Out;

pub(super) fn contracts(asked: &Asked) -> usize {
    asked.all().iter().filter(|asked| asked.ends_with(": contract --json")).count()
}

fn slow_contract() -> FakeTransport {
    serving(fine()).script(CONTRACT, |_, mut out| async move {
        time::sleep(Duration::from_secs(1)).await;
        out.send(Out::Stdout(served())).await.map_err(|_| Error::OutputClosed)?;
        Ok(exit(0, ""))
    })
}

pub(super) fn once(fake: FakeTransport, argv: &[&str], json: &str) -> FakeTransport {
    let calls = AtomicUsize::new(0);
    let first = line(json);
    fake.script(argv, move |_, mut out| {
        let lines: Vec<String> =
            (calls.fetch_add(1, Ordering::SeqCst) == 0).then(|| first.clone()).into_iter().collect();
        async move {
            for line in lines {
                out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)?;
            }
            future::pending().await
        }
    })
}

fn since_of_the_first(store: &Store) -> Option<String> {
    store.agents().expect("the agents once connected")[0].since.clone()
}

fn notices() -> Connection {
    Connection::Missing(vec![Missing { name: "notices".to_owned(), kind: Kind::Feature, reason: None }])
}

mod resuming {
    use super::*;

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn skips_the_wait_while_retrying(#[values(Resume::Foreground, Resume::Network)] resume: Resume) {
        let (store, mut changes) = spawned(refusing(Error::Timeout), &Asked::default());
        retrying(&mut changes).await;
        let started = Instant::now();
        poke(&store, resume);
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn closes_the_streams_once_connected_and_opens_them_again(
        #[values(Resume::Foreground, Resume::Network)] resume: Resume,
    ) {
        let alive = Arc::new(());
        let fake = held(held(serving(fine()), EVENTS, &alive), APP, &alive);
        let idle = Arc::strong_count(&alive);
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        until_connected(&mut changes).await;
        poke(&store, resume);
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        until_connected(&mut changes).await;
        assert_eq!(Arc::strong_count(&alive), idle + 2);
        assert_eq!(contracts(&asked), 2);
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn keeps_what_it_knew_while_it_connects_again(#[values(Resume::Foreground, Resume::Network)] resume: Resume) {
        let (store, mut changes) = spawned(once(serving(fine()), STATUS, WATCH), &Asked::default());
        until_connected(&mut changes).await;
        poke(&store, resume);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Connecting]);
        assert_eq!(store.session(), Some(serde_json::from_str(WATCH).expect("the watch fixture")));
        assert!(store.app().is_some());
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn forgets_since_when_each_agent_is_in_its_status(
        #[values(Resume::Foreground, Resume::Network)] resume: Resume,
    ) {
        let lines = [Vec::new(), vec![watched(7, "waiting")], fine()[2].clone()];
        let fake = once(serving(lines), EVENTS, &status_event("waiting"));
        let (store, mut changes) = spawned(fake, &Asked::default());
        until_connected(&mut changes).await;
        time::sleep(Duration::from_secs(1)).await;
        assert!(since_of_the_first(&store).is_some());
        unread(&mut changes);
        poke(&store, resume);
        time::sleep(Duration::from_secs(60)).await;
        let changes = unread(&mut changes);
        assert_eq!(connections(&changes), connected());
        assert_eq!(since_of_the_first(&store), None);
        assert!(changes.contains(&Change::Session));
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn says_some_events_may_be_lost_only_when_it_opens_them_again(
        #[values(Resume::Foreground, Resume::Network)] resume: Resume,
    ) {
        let reopened = |changes: &[Change]| changes.iter().filter(|&change| *change == Change::Reopened).count();
        let (store, mut changes) = spawned(serving(fine()), &Asked::default());
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(reopened(&unread(&mut changes)), 0);
        poke(&store, resume);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(reopened(&unread(&mut changes)), 1);
    }

    #[rstest]
    #[case::missing_something(contract(&without_notices()), notices())]
    #[case::unauthorized(refusing(Error::Unauthorized), Connection::Unauthorized)]
    #[case::incompatible(
        contract(r#"{"contract":2}"#),
        Connection::Incompatible(Error::ContractTooNew { server: 2, app: 1 })
    )]
    #[tokio::test(start_paused = true)]
    async fn tries_once_more_after_stopping(
        #[case] fake: FakeTransport,
        #[case] stopped: Connection,
        #[values(Resume::Foreground, Resume::Network)] resume: Resume,
    ) {
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Connecting, stopped.clone()]);
        poke(&store, resume);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Connecting, stopped]);
        assert_eq!(contracts(&asked), 2);
    }

    #[rstest]
    #[case::foreground_while_retrying(refusing(Error::Timeout), Resume::Foreground, 2)]
    #[case::foreground_once_connected(serving(fine()), Resume::Foreground, 2)]
    #[case::foreground_while_connecting(slow_contract(), Resume::Foreground, 1)]
    #[case::network_while_retrying(refusing(Error::Timeout), Resume::Network, 2)]
    #[case::network_once_connected(serving(fine()), Resume::Network, 2)]
    #[case::network_while_connecting(slow_contract(), Resume::Network, 2)]
    #[tokio::test(start_paused = true)]
    async fn called_again_and_again_starts_a_single_attempt(
        #[case] fake: FakeTransport,
        #[case] resume: Resume,
        #[case] expected: usize,
    ) {
        let asked = Asked::default();
        let (store, _changes) = spawned(fake, &asked);
        time::sleep(Duration::from_millis(500)).await;
        for _ in 0..5 {
            poke(&store, resume);
        }
        time::sleep(Duration::from_millis(1500)).await;
        assert_eq!(contracts(&asked), expected);
        assert_ne!(store.connection(), Connection::Connecting);
    }
}

mod an_attempt_under_way {
    use super::*;

    fn silent() -> FakeTransport {
        serving(fine()).script(CONTRACT, |_, _| future::pending())
    }

    #[tokio::test(start_paused = true)]
    async fn waits_for_the_first_snapshot_when_the_app_comes_back_to_the_foreground() {
        let asked = Asked::default();
        let (store, mut changes) = spawned(late(serving(fine())), &asked);
        time::sleep(Duration::from_millis(500)).await;
        store.resume_now();
        until_connected(&mut changes).await;
        assert_eq!(contracts(&asked), 1);
    }

    #[rstest]
    #[case::choosing_a_route(slow_contract())]
    #[case::reading_a_contract_that_never_comes(silent())]
    #[case::waiting_for_the_first_snapshot(late(serving(fine())))]
    #[tokio::test(start_paused = true)]
    async fn is_dropped_for_another_at_once_when_the_network_changes(#[case] fake: FakeTransport) {
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        time::sleep(Duration::from_millis(500)).await;
        unread(&mut changes);
        let started = Instant::now();
        store.network_changed();
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        assert_eq!((started.elapsed(), contracts(&asked)), (Duration::ZERO, 2));
    }
}
