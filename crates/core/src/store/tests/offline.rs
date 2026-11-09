use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};

use rstest::rstest;

use super::boot::late;
use super::connect::{contract, refusing};
use super::resume::{contracts, once};
use super::retry::{answered, closing_later, held};
use super::streams::{connected, connections, fine, line, serving};
use super::*;
use crate::cli::status::fixtures::WATCH;
use crate::store::streams::{APP, EVENTS, STATUS};

async fn gone_offline(store: &Store, changes: &mut Changes) -> Duration {
    let started = Instant::now();
    store.offline();
    assert_eq!(next_connection(changes).await, Connection::Offline);
    started.elapsed()
}

fn connecting_once() -> FakeTransport {
    let calls = AtomicUsize::new(0);
    serving(fine()).script(STATUS, move |_, out| {
        let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
        async move {
            answered(out, first.then(|| line(WATCH))).await?;
            future::pending().await
        }
    })
}

mod going_offline {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn closes_the_streams_and_keeps_the_session() {
        let alive = Arc::new(());
        let fake = held(held(once(serving(fine()), STATUS, WATCH), EVENTS, &alive), APP, &alive);
        let idle = Arc::strong_count(&alive);
        let (store, mut changes) = spawned(fake, &Asked::default());
        until_connected(&mut changes).await;
        assert_eq!(gone_offline(&store, &mut changes).await, Duration::ZERO);
        time::sleep(Duration::from_secs(1)).await;
        assert_eq!(Arc::strong_count(&alive), idle);
        assert_eq!(store.session(), Some(serde_json::from_str(WATCH).expect("the watch fixture")));
    }

    #[rstest]
    #[case::once_connected(serving(fine()))]
    #[case::while_connecting(late(serving(fine())))]
    #[case::while_retrying(refusing(Error::Timeout))]
    #[case::after_stopping(contract(r#"{"contract":2}"#))]
    #[tokio::test(start_paused = true)]
    async fn tries_nothing_more_while_the_network_is_down(#[case] fake: FakeTransport) {
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        time::sleep(Duration::from_millis(500)).await;
        unread(&mut changes);
        let tried = contracts(&asked);
        assert_eq!(gone_offline(&store, &mut changes).await, Duration::ZERO);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(unread(&mut changes), []);
        assert_eq!((store.connection(), contracts(&asked)), (Connection::Offline, tried));
    }

    #[rstest]
    #[case::network(Resume::Network)]
    #[case::offline(Resume::Offline)]
    #[tokio::test(start_paused = true)]
    async fn stays_there_until_the_network_is_back(#[case] resume: Resume) {
        let asked = Asked::default();
        let (store, mut changes) = spawned(refusing(Error::Timeout), &asked);
        retrying(&mut changes).await;
        gone_offline(&store, &mut changes).await;
        poke(&store, resume);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(unread(&mut changes), []);
        assert_eq!(contracts(&asked), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn wins_over_an_attempt_asked_just_before() {
        let asked = Asked::default();
        let (store, mut changes) = spawned(contract(r#"{"contract":2}"#), &asked);
        time::sleep(Duration::from_secs(1)).await;
        unread(&mut changes);
        store.resume_now();
        store.offline();
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Offline]);
        assert_eq!(contracts(&asked), 1);
    }
}

mod coming_back_online {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn tries_at_once_and_connects() {
        let asked = Asked::default();
        let (store, mut changes) = spawned(serving(fine()), &asked);
        until_connected(&mut changes).await;
        gone_offline(&store, &mut changes).await;
        let started = Instant::now();
        store.online();
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        assert_eq!(started.elapsed(), Duration::ZERO);
        until_connected(&mut changes).await;
        assert_eq!(contracts(&asked), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn waits_a_second_again_after_failing() {
        let (store, mut changes) = spawned(refusing(Error::Timeout), &Asked::default());
        let waits = [next_wait(&mut changes).await, next_wait(&mut changes).await, next_wait(&mut changes).await];
        assert_eq!(waits, [1, 2, 4].map(Duration::from_secs));
        gone_offline(&store, &mut changes).await;
        store.online();
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        assert_eq!(next_wait(&mut changes).await, Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn tries_when_it_was_back_before_the_attempt() {
        let asked = Asked::default();
        let (store, mut changes) = spawned(contract(r#"{"contract":2}"#), &asked);
        time::sleep(Duration::from_secs(1)).await;
        unread(&mut changes);
        store.resume_now();
        store.offline();
        store.online();
        time::sleep(Duration::from_secs(60)).await;
        assert!(!connections(&unread(&mut changes)).contains(&Connection::Offline));
        assert_eq!(contracts(&asked), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn right_after_going_offline_tries_once() {
        let asked = Asked::default();
        let (store, mut changes) = spawned(serving(fine()), &asked);
        until_connected(&mut changes).await;
        store.offline();
        store.online();
        time::sleep(Duration::from_secs(60)).await;
        let expected: Vec<Connection> = iter::once(Connection::Offline).chain(connected()).collect();
        assert_eq!(connections(&unread(&mut changes)), expected);
        assert_eq!(contracts(&asked), 2);
    }
}

mod coming_back_to_the_foreground {
    use super::*;

    #[rstest]
    #[case::once_retrying(refusing(Error::Timeout))]
    #[case::once_connected(connecting_once())]
    #[tokio::test(start_paused = true)]
    async fn tries_once_and_goes_back_offline_after_failing(#[case] fake: FakeTransport) {
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        time::sleep(Duration::from_millis(500)).await;
        unread(&mut changes);
        gone_offline(&store, &mut changes).await;
        store.resume_now();
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), [Connection::Connecting, Connection::Offline]);
        assert_eq!(contracts(&asked), 2);
        store.online();
        assert_eq!(next_connection(&mut changes).await, Connection::Connecting);
        assert_eq!(next_wait(&mut changes).await, Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn connects_when_the_network_was_up_after_all() {
        let asked = Asked::default();
        let (store, mut changes) = spawned(serving(fine()), &asked);
        until_connected(&mut changes).await;
        gone_offline(&store, &mut changes).await;
        store.resume_now();
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), connected());
        assert_eq!(contracts(&asked), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn retries_again_once_connected() {
        let (store, mut changes) = spawned(closing_later(serving(fine())), &Asked::default());
        until_connected(&mut changes).await;
        gone_offline(&store, &mut changes).await;
        store.resume_now();
        until_connected(&mut changes).await;
        assert_eq!(next_wait(&mut changes).await, Duration::from_secs(1));
    }
}
