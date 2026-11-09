use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::SinkExt;
use rstest::rstest;

use super::resume::{contracts, once};
use super::retry::{answered, exit, held};
use super::streams::{BOOT, Lines, connected, connections, contract_of, fine, line, served, serving, snapshot};
use super::*;
use crate::board::Since;
use crate::cli::status::fixtures::{APP as APP_SNAPSHOT, WATCH};
use crate::queue::Outcome;
use crate::store::streams::{APP, EVENTS, STATUS};
use crate::transport::Out;

const REQUESTS: &[&str] = &["requests", "--follow", "-"];
const RESTARTED: &str = "4243-1760600100";

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

fn by_call<S, F>(fake: FakeTransport, argv: &[&str], script: S) -> FakeTransport
where
    S: Fn(usize, OutSink) -> F + Send + Sync + 'static,
    F: Future<Output = Result<Exit, Error>> + Send + 'static,
{
    let calls = AtomicUsize::new(0);
    fake.script(argv, move |_, out| script(calls.fetch_add(1, Ordering::SeqCst), out))
}

fn rebooting_after(fake: FakeTransport, wait: Duration) -> FakeTransport {
    by_call(fake, CONTRACT, move |call, out| async move {
        if call == 0 {
            return answered(out, Some(served())).await;
        }
        time::sleep(wait).await;
        answered(out, Some(contract_of(RESTARTED))).await
    })
}

fn rebooting(fake: FakeTransport) -> FakeTransport {
    rebooting_after(fake, Duration::ZERO)
}

pub(super) fn answering_only(fake: FakeTransport, first: bool) -> FakeTransport {
    by_call(fake, CONTRACT, move |call, out| answered(out, ((call == 0) == first).then(served)))
}

pub(super) fn late(fake: FakeTransport) -> FakeTransport {
    fake.script(STATUS, |_, mut out| async move {
        time::sleep(Duration::from_secs(5)).await;
        said(&mut out, line(WATCH)).await?;
        future::pending().await
    })
}

fn stale_at_first(fake: FakeTransport) -> FakeTransport {
    by_call(fake, CONTRACT, |call, out| answered(out, Some(if call == 0 { contract_of(RESTARTED) } else { served() })))
}

async fn said(out: &mut OutSink, line: String) -> Result<(), Error> {
    out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)
}

async fn watched(call: usize, mut out: OutSink, after: Option<String>, ends: bool) -> Result<Exit, Error> {
    if call > 0 {
        return future::pending().await;
    }
    said(&mut out, line(WATCH)).await?;
    time::sleep(Duration::from_secs(5)).await;
    if let Some(after) = after {
        said(&mut out, after).await?;
    }
    if ends { Ok(exit(0, "")) } else { future::pending().await }
}

fn restarting(fake: FakeTransport, after: Option<String>, ends: bool) -> FakeTransport {
    let watching = move |call, out| watched(call, out, after.clone(), ends);
    held(by_call(once(fake, APP, APP_SNAPSHOT), STATUS, watching), REQUESTS, &Arc::default())
}

fn restarted() -> String {
    snapshot(RESTARTED, 1, "restarted")
}

fn at_once() -> Vec<Connection> {
    vec![Connection::Connecting]
}

fn retried() -> Vec<Connection> {
    vec![Connection::Retrying { r#in: Duration::from_secs(1), error: Error::StreamEnded }, Connection::Connecting]
}

mod a_restart {
    use super::*;

    #[rstest]
    #[case::in_the_contract(restarting(rebooting(serving(fine())), None, true), retried())]
    #[case::in_a_snapshot(restarting(rebooting(serving(fine())), Some(restarted()), false), at_once())]
    #[case::in_the_last_snapshot_of_a_session(
        restarting(rebooting(serving(fine())), Some(restarted()), true),
        at_once()
    )]
    #[tokio::test(start_paused = true)]
    async fn forgets_the_session_the_app_and_what_was_queued(
        #[case] fake: FakeTransport,
        #[case] expected: Vec<Connection>,
    ) {
        let asked = Asked::default();
        let (store, mut changes) = spawned(fake, &asked);
        until_connected(&mut changes).await;
        let queue = store.queue().expect("a queue once connected");
        let (hung, queued) = (queue.ask(words(&["focus"]), None), queue.ask(words(&["status"]), None));
        time::sleep(Duration::from_secs(1)).await;
        unread(&mut changes);
        time::sleep(Duration::from_secs(60)).await;
        let changes = unread(&mut changes);
        assert_eq!(connections(&changes), expected);
        assert!(changes.contains(&Change::Session) && changes.contains(&Change::App));
        assert_eq!((store.session(), store.agents(), store.app()), (None, None, None));
        let replies = (hung.now_or_never(), queued.now_or_never());
        assert_eq!(replies, (Some(Err(Error::Unanswered)), Some(Ok(Outcome::Skipped))));
        assert_eq!(contracts(&asked), 2);
    }

    #[test]
    fn forgets_since_when_each_pane_was_in_its_status() {
        let shared = Shared::default();
        shared.restarted(Boot(BOOT.to_owned()));
        shared.state().since.heard(4, Some("waiting"), "2026-10-16T09:30:00.123Z");
        assert!(shared.restarted(Boot(RESTARTED.to_owned())));
        assert_eq!(shared.state().since, Since::default());
    }

    #[tokio::test(start_paused = true)]
    async fn in_a_snapshot_moves_the_queue_before_the_contract_answers() {
        let slow = rebooting_after(serving(fine()), Duration::from_secs(30));
        let (store, mut changes) = spawned(restarting(slow, Some(restarted()), false), &Asked::default());
        until_connected(&mut changes).await;
        let queue = store.queue().expect("a queue once connected");
        let (hung, queued) = (queue.ask(words(&["focus"]), None), queue.ask(words(&["status"]), None));
        time::sleep(Duration::from_secs(10)).await;
        assert_eq!(store.connection(), Connection::Connecting);
        let replies = (hung.now_or_never(), queued.now_or_never());
        assert_eq!(replies, (Some(Err(Error::Unanswered)), Some(Ok(Outcome::Skipped))));
    }

    #[tokio::test(start_paused = true)]
    async fn in_the_first_snapshot_is_seen_once() {
        let asked = Asked::default();
        let fake = held(late(stale_at_first(serving(fine()))), REQUESTS, &Arc::default());
        let (store, mut changes) = spawned(fake, &asked);
        time::sleep(Duration::from_secs(1)).await;
        let hung = store.queue().expect("a queue once the contract answered").ask(words(&["focus"]), None);
        until_connected(&mut changes).await;
        assert_eq!((hung.now_or_never(), contracts(&asked)), (Some(Err(Error::Unanswered)), 2));
        unread(&mut changes);
        store.resume_now();
        time::sleep(Duration::from_secs(60)).await;
        let changes = unread(&mut changes);
        assert_eq!(connections(&changes), connected());
        assert!(!changes.contains(&Change::Session));
        assert_eq!(store.session(), Some(serde_json::from_str(WATCH).expect("the watch fixture")));
    }

    #[tokio::test(start_paused = true)]
    async fn seen_twice_without_a_snapshot_between_waits_out_the_backoff() {
        let asked = Asked::default();
        let fake = serving(fine()).answer(CONTRACT, &[&contract_of(RESTARTED)], &[], 0);
        let (_store, mut changes) = spawned(fake, &asked);
        let started = Instant::now();
        let waits = [retrying(&mut changes).await, retrying(&mut changes).await];
        let disagree = Error::StaleBoot("the server's contract and its snapshots name different boots".to_owned());
        assert_eq!(waits, [(Duration::from_secs(1), disagree.clone()), (Duration::from_secs(2), disagree)]);
        assert_eq!((started.elapsed(), contracts(&asked)), (Duration::from_secs(1), 3));
    }
}

mod the_queue {
    use super::*;

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn moves_to_another_route_and_keeps_the_session(
        #[values(Resume::Foreground, Resume::Network)] resume: Resume,
    ) {
        let asked = Asked::default();
        let lan = held(answering_only(serving(fine()), true), REQUESTS, &Arc::default());
        let tailnet = held(answering_only(serving(Lines::default()), false), REQUESTS, &Arc::default());
        let (store, driver) = store_on(vec![route(LAN), route(TAILNET)], vec![(LAN, lan), (TAILNET, tailnet)], &asked);
        let mut changes = store.changes();
        tokio::spawn(driver);
        until_connected(&mut changes).await;
        let queue = store.queue().expect("a queue once connected");
        let hung = queue.ask(words(&["focus"]), None);
        time::sleep(Duration::from_secs(1)).await;
        poke(&store, resume);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(hung.now_or_never(), Some(Err(Error::Unanswered)));
        queue.run(words(&["status"]), None);
        time::sleep(Duration::from_secs(1)).await;
        assert!(asked.all().contains(&format!("{TAILNET}: requests --follow -")));
        assert_eq!(store.session(), Some(serde_json::from_str(WATCH).expect("the watch fixture")));
    }

    #[tokio::test(start_paused = true)]
    async fn warns_of_a_command_that_failed() {
        let fake = serving(fine()).answer(REQUESTS, &[], &[], 0).answer(&["focus"], &[], &["it failed"], 1);
        let (store, mut changes) = spawned(fake, &Asked::default());
        until_connected(&mut changes).await;
        store.queue().expect("a queue once connected").run(words(&["focus"]), None);
        time::sleep(Duration::from_secs(1)).await;
        let failed = Change::Warning(Error::Failed { code: 1, stderr: "it failed".to_owned() });
        assert!(unread(&mut changes).contains(&failed));
    }
}

mod dropping_the_store {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn cancels_the_streams_and_the_queue() {
        let alive = Arc::new(());
        let fake = held(held(held(serving(fine()), EVENTS, &alive), APP, &alive), REQUESTS, &alive);
        let idle = Arc::strong_count(&alive);
        let (store, driver) = store_on(vec![route(LAN)], vec![(LAN, fake)], &Asked::default());
        let mut changes = store.changes();
        let driver = tokio::spawn(driver);
        until_connected(&mut changes).await;
        let hung = store.queue().expect("a queue once connected").ask(words(&["focus"]), None);
        time::sleep(Duration::from_secs(1)).await;
        assert_eq!(Arc::strong_count(&alive), idle + 3);
        drop(store);
        driver.await.expect("the driver does not panic");
        assert_eq!(Arc::strong_count(&alive), 1);
        assert_eq!(hung.now_or_never(), Some(Err(Error::Unanswered)));
    }
}
