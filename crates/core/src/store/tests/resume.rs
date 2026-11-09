use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::SinkExt;
use rstest::rstest;

use super::connect::{contract, refusing, without_notices};
use super::retry::{exit, held};
use super::streams::{connections, fine, line, served, serving};
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

pub(super) fn once(fake: FakeTransport, argv: &[&str], json: &'static str) -> FakeTransport {
    let calls = AtomicUsize::new(0);
    fake.script(argv, move |_, mut out| {
        let lines: Vec<String> = (calls.fetch_add(1, Ordering::SeqCst) == 0).then(|| line(json)).into_iter().collect();
        async move {
            for line in lines {
                out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)?;
            }
            future::pending().await
        }
    })
}

fn notices() -> Connection {
    Connection::Missing(vec![Missing { name: "notices".to_owned(), kind: Kind::Feature, reason: None }])
}

mod resume_now {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn skips_the_wait_while_retrying() {
        let (store, mut changes) = spawned(refusing(Error::Timeout), &Asked::default());
        retrying(&mut changes).await;
        let started = Instant::now();
        store.resume_now();
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn closes_the_streams_once_connected_and_opens_them_again() {
        let alive = Arc::new(());
        let fake = held(held(serving(fine()), EVENTS, &alive), APP, &alive);
        let idle = Arc::strong_count(&alive);
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        until_connected(&mut changes).await;
        store.resume_now();
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        until_connected(&mut changes).await;
        assert_eq!(Arc::strong_count(&alive), idle + 2);
        assert_eq!(contracts(&asked), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_what_it_knew_while_it_connects_again() {
        let (store, mut changes) = spawned(once(serving(fine()), STATUS, WATCH), &Asked::default());
        until_connected(&mut changes).await;
        store.resume_now();
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Connecting]);
        assert_eq!(store.session(), Some(serde_json::from_str(WATCH).expect("the watch fixture")));
        assert!(store.app().is_some());
    }

    #[rstest]
    #[case::missing_something(contract(&without_notices()), notices())]
    #[case::unauthorized(refusing(Error::Unauthorized), Connection::Unauthorized)]
    #[case::incompatible(
        contract(r#"{"contract":2}"#),
        Connection::Incompatible(Error::ContractTooNew { server: 2, app: 1 })
    )]
    #[tokio::test(start_paused = true)]
    async fn tries_once_more_after_stopping(#[case] fake: FakeTransport, #[case] stopped: Connection) {
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Connecting, stopped.clone()]);
        store.resume_now();
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Connecting, stopped]);
        assert_eq!(contracts(&asked), 2);
    }

    #[rstest]
    #[case::while_retrying(refusing(Error::Timeout), 2)]
    #[case::once_connected(serving(fine()), 2)]
    #[case::while_connecting(slow_contract(), 1)]
    #[tokio::test(start_paused = true)]
    async fn called_again_and_again_starts_a_single_attempt(#[case] fake: FakeTransport, #[case] expected: usize) {
        let asked = Asked::default();
        let (store, _changes) = spawned(fake, &asked);
        time::sleep(Duration::from_millis(500)).await;
        for _ in 0..5 {
            store.resume_now();
        }
        time::sleep(Duration::from_secs(1)).await;
        assert_eq!(contracts(&asked), expected);
        assert_ne!(store.connection(), Connection::Connecting);
    }
}
