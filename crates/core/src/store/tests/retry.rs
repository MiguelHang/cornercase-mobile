use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::SinkExt;
use rstest::rstest;

use super::streams::{fine, line, serving};
use super::*;
use crate::cli::status::fixtures::{APP as APP_SNAPSHOT, WATCH};
use crate::store::streams::{APP, EVENTS, STATUS};
use crate::transport::Out;

pub(super) fn exit(code: i32, stderr: &str) -> Exit {
    Exit { code, stderr: stderr.to_owned() }
}

fn ending(argv: &[&str], end: Result<Exit, Error>) -> FakeTransport {
    serving(fine()).script(argv, move |_, _| future::ready(end.clone()))
}

pub(super) fn held(fake: FakeTransport, argv: &[&str], alive: &Arc<()>) -> FakeTransport {
    let alive = alive.clone();
    fake.script(argv, move |stdin, out| {
        let held = (alive.clone(), stdin, out);
        async move {
            let _held = held;
            future::pending().await
        }
    })
}

fn failing_but(snapshot_at: usize) -> FakeTransport {
    let calls = AtomicUsize::new(0);
    serving(fine()).script(STATUS, move |_, out| {
        let call = calls.fetch_add(1, Ordering::SeqCst);
        answered(out, (call == snapshot_at).then(|| line(WATCH)))
    })
}

fn closing_later(fake: FakeTransport) -> FakeTransport {
    fake.script(STATUS, |_, out| async {
        let exit = answered(out, Some(line(WATCH))).await;
        time::sleep(Duration::from_secs(5)).await;
        exit
    })
}

pub(super) async fn answered(mut out: OutSink, line: Option<String>) -> Result<Exit, Error> {
    let line = line.ok_or(Error::Dropped)?;
    out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)?;
    Ok(exit(0, ""))
}

async fn waits(fake: FakeTransport, count: usize) -> Vec<u64> {
    let (_store, mut changes) = spawned(fake, &Asked::default());
    let mut waits = Vec::new();
    for _ in 0..count {
        waits.push(next_wait(&mut changes).await.as_secs());
    }
    waits
}

mod a_stream_that_ends {
    use super::*;

    #[rstest]
    #[case::events_that_exit_cleanly(EVENTS, Ok(exit(0, "")), Error::StreamEnded)]
    #[case::session_that_exits_with_an_error(STATUS, Ok(exit(1, "boom")), Error::Failed { code: 1, stderr: "boom".to_owned() })]
    #[case::app_that_exits_with_an_error(APP, Ok(exit(2, "bad")), Error::Failed { code: 2, stderr: "bad".to_owned() })]
    #[case::events_whose_connection_drops(EVENTS, Err(Error::Dropped), Error::Dropped)]
    #[case::app_whose_connection_goes_silent(APP, Err(Error::Timeout), Error::Timeout)]
    #[tokio::test(start_paused = true)]
    async fn retries_with_its_error(#[case] argv: &[&str], #[case] end: Result<Exit, Error>, #[case] error: Error) {
        let (_store, mut changes) = spawned(ending(argv, end), &Asked::default());
        assert_eq!(retrying(&mut changes).await, (Duration::from_secs(1), error));
    }

    #[tokio::test(start_paused = true)]
    async fn closes_the_other_two() {
        let alive = Arc::new(());
        let fake = held(held(closing_later(serving(fine())), EVENTS, &alive), APP, &alive);
        let idle = Arc::strong_count(&alive);
        let (_store, mut changes) = spawned(fake, &Asked::default());
        until_connected(&mut changes).await;
        assert_eq!(Arc::strong_count(&alive), idle + 2);
        retrying(&mut changes).await;
        assert_eq!(Arc::strong_count(&alive), idle);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_session_and_the_app_while_retrying() {
        let (store, mut changes) = spawned(closing_later(serving(fine())), &Asked::default());
        retrying(&mut changes).await;
        assert!(matches!(store.connection(), Connection::Retrying { .. }));
        assert_eq!(store.session(), Some(serde_json::from_str(WATCH).expect("the watch fixture")));
        assert_eq!(store.app(), Some(serde_json::from_str(APP_SNAPSHOT).expect("the app fixture")));
    }
}

mod the_waits {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn keep_growing_while_the_session_fails_at_once() {
        let started = Instant::now();
        assert_eq!(waits(failing_but(usize::MAX), 6).await, [1, 2, 4, 8, 15, 15]);
        assert_eq!(started.elapsed(), Duration::from_secs(30));
    }

    #[tokio::test(start_paused = true)]
    async fn start_again_from_one_second_only_after_a_snapshot() {
        assert_eq!(waits(failing_but(3), 6).await, [1, 2, 4, 1, 2, 4]);
    }
}
