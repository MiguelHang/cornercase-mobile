use std::future;

use futures_util::SinkExt;
use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::contract::fixtures::REAL;
use crate::cli::events::Event;
use crate::cli::events::fixtures as events;
use crate::cli::status::fixtures as snapshots;
use crate::store::streams::{APP, EVENTS, STATUS};
use crate::transport::Out;

const SECRET: &str = "c2VjcmV0";
pub(super) const BOOT: &str = "4242-1760600000";

pub(super) type Lines = [Vec<String>; 3];

pub(super) fn line(json: &str) -> String {
    serde_json::from_str::<Value>(json).expect("a fixture is JSON").to_string()
}

pub(super) fn fine() -> Lines {
    [vec![line(events::OPENED), line(events::EXITED)], vec![line(snapshots::WATCH)], vec![line(snapshots::APP)]]
}

pub(super) fn contract_of(boot: &str) -> String {
    let mut contract: Value = serde_json::from_str(REAL).expect("the real contract parses");
    contract["boot"] = boot.into();
    contract.to_string()
}

pub(super) fn served() -> String {
    contract_of(BOOT)
}

pub(super) fn serving(lines: Lines) -> FakeTransport {
    let fake = FakeTransport::default().answer(CONTRACT, &[&served()], &[], 0);
    [EVENTS, STATUS, APP].into_iter().zip(lines).fold(fake, |fake, (argv, lines)| {
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        fake.open(argv, &lines)
    })
}

async fn started(lines: Lines) -> (Store, Vec<Change>) {
    let (store, driver) = store_on(vec![route(LAN)], vec![(LAN, serving(lines))], &Asked::default());
    let changes = settled(&store, driver).await;
    (store, changes)
}

pub(super) fn connections(changes: &[Change]) -> Vec<Connection> {
    changes
        .iter()
        .filter_map(|change| if let Change::Connection(it) = change { Some(it.clone()) } else { None })
        .collect()
}

fn heard(changes: &[Change]) -> Vec<Event> {
    changes.iter().filter_map(|change| if let Change::Event(it) = change { Some(*it.clone()) } else { None }).collect()
}

fn warnings(changes: &[Change]) -> Vec<Error> {
    changes.iter().filter_map(|change| if let Change::Warning(it) = change { Some(it.clone()) } else { None }).collect()
}

fn event(json: &str) -> Event {
    serde_json::from_str(json).expect("an event fixture")
}

pub(super) fn connected() -> Vec<Connection> {
    vec![Connection::Connecting, Connection::Connected { route: route(LAN).address().clone() }]
}

pub(super) fn snapshot(boot: &str, revision: u64, version: &str) -> String {
    json!({"boot": boot, "revision": revision, "version": version}).to_string()
}

pub(super) fn status_event(to: &str) -> String {
    let mut event: Value = serde_json::from_str(events::STATUS).expect("the status event fixture");
    event["to"] = to.into();
    event.to_string()
}

pub(super) fn watched(revision: u64, status: &str) -> String {
    let mut watch: Value = serde_json::from_str(snapshots::WATCH).expect("the watch fixture");
    watch["revision"] = revision.into();
    watch["projects"][0]["workspaces"][0]["tabs"][0]["panes"][0]["status"] = status.into();
    watch.to_string()
}

fn later(fake: FakeTransport, argv: &[&str], lines: Vec<String>) -> FakeTransport {
    fake.script(argv, move |_, mut out| {
        let lines = lines.clone();
        async move {
            time::sleep(Duration::from_secs(5)).await;
            for line in lines {
                out.send(Out::Stdout(line)).await.map_err(|_| Error::OutputClosed)?;
            }
            future::pending().await
        }
    })
}

async fn agents_once(lines: Lines, late: usize) -> Vec<(u64, Option<String>, Option<String>)> {
    let argv = [EVENTS, STATUS, APP][late];
    let fake = later(serving(lines.clone()), argv, lines[late].clone());
    let (store, driver) = store_on(vec![route(LAN)], vec![(LAN, fake)], &Asked::default());
    settled(&store, driver).await;
    let agents = store.agents().expect("the agents once there is a session");
    agents.into_iter().map(|row| (row.pane, row.status, row.since)).collect()
}

fn assert_fine(store: &Store, changes: &[Change]) {
    assert_eq!(connections(changes), connected());
    assert_eq!(store.session(), Some(serde_json::from_str(snapshots::WATCH).expect("the watch fixture")));
    assert_eq!(store.app(), Some(serde_json::from_str(snapshots::APP).expect("the app fixture")));
    assert_eq!(heard(changes), [event(events::OPENED), event(events::EXITED)]);
}

mod the_start {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn asks_for_the_three_streams_in_order_after_the_contract() {
        let asked = Asked::default();
        let (store, driver) = store_on(vec![route(LAN)], vec![(LAN, serving(fine()))], &asked);
        settled(&store, driver).await;
        let expected =
            ["contract --json", "events --json --notices", "status --follow --json", "status --app --follow --json"];
        assert_eq!(asked.all(), expected.map(|argv| format!("{LAN}: {argv}")));
    }

    #[tokio::test(start_paused = true)]
    async fn connects_with_the_session_the_app_and_the_events() {
        let (store, changes) = started(fine()).await;
        assert_fine(&store, &changes);
        assert!(changes.contains(&Change::Session) && changes.contains(&Change::App));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_connecting_until_the_first_snapshot() {
        let [events, _, app] = fine();
        let (store, changes) = started([events, Vec::new(), app]).await;
        assert_eq!(connections(&changes), [Connection::Connecting]);
        assert_eq!(store.connection(), Connection::Connecting);
        assert_eq!(store.session(), None);
    }
}

mod the_session {
    use super::*;

    #[rstest]
    #[case::older_of_the_same_boot(snapshot(BOOT, 6, "second"), "first")]
    #[case::as_old_of_the_same_boot(snapshot(BOOT, 7, "second"), "first")]
    #[case::newer_of_the_same_boot(snapshot(BOOT, 8, "second"), "second")]
    #[tokio::test(start_paused = true)]
    async fn takes_a_snapshot_only_if_newer_in_its_boot(#[case] next: String, #[case] kept: &str) {
        let [events, _, app] = fine();
        let (store, changes) = started([events, vec![snapshot(BOOT, 7, "first"), next], app]).await;
        assert_eq!(store.session().map(|session| session.version), Some(kept.to_owned()));
        assert_eq!(connections(&changes), connected());
    }

    #[tokio::test(start_paused = true)]
    async fn skips_a_line_without_a_boot_with_a_warning() {
        let [events, _, app] = fine();
        let unbooted = json!({"foo": 1}).to_string();
        let lines = vec![snapshot(BOOT, 7, "first"), unbooted, snapshot(BOOT, 8, "second")];
        let (store, changes) = started([events, lines, app]).await;
        assert!(matches!(&warnings(&changes)[..], [Error::Json(reason)] if reason.contains("without its boot")));
        assert_eq!(store.session().map(|session| session.version), Some("second".to_owned()));
        assert_eq!(connections(&changes), connected());
    }
}

mod a_line_that_does_not_parse {
    use super::*;

    #[rstest]
    #[case::in_the_events(0, format!(r#"{{"token": {SECRET}"#))]
    #[case::in_the_session(1, format!(r#"{{"token": {SECRET}"#))]
    #[case::in_the_app(2, format!(r#"{{"token": {SECRET}"#))]
    #[case::of_another_shape_in_the_session(1, format!(r#"{{"revision": "{SECRET}"}}"#))]
    #[tokio::test(start_paused = true)]
    async fn is_a_warning_without_its_text_and_the_stream_goes_on(#[case] stream: usize, #[case] broken: String) {
        let mut lines = fine();
        lines[stream].insert(0, broken);
        let (store, changes) = started(lines).await;
        assert!(matches!(&warnings(&changes)[..], [Error::Json(reason)] if !reason.contains(SECRET)));
        assert_fine(&store, &changes);
    }
}

mod the_agents {
    use super::*;

    const TIME: &str = "2026-10-16T09:30:00.123Z";

    fn agent(pane: u64, status: &str, since: Option<&str>) -> (u64, Option<String>, Option<String>) {
        (pane, Some(status.to_owned()), since.map(str::to_owned))
    }

    #[tokio::test(start_paused = true)]
    async fn are_none_before_the_first_snapshot() {
        let [events, _, app] = fine();
        let (store, _) = started([events, Vec::new(), app]).await;
        assert_eq!(store.agents(), None);
    }

    #[rstest]
    #[case::before_the_snapshot(1)]
    #[case::after_the_snapshot(0)]
    #[tokio::test(start_paused = true)]
    async fn take_the_time_of_a_status_event_heard(#[case] late: usize) {
        let lines = [vec![status_event("working")], vec![line(snapshots::WATCH)], vec![line(snapshots::APP)]];
        assert_eq!(agents_once(lines, late).await, [agent(4, "working", Some(TIME)), agent(5, "working", None)]);
    }

    #[tokio::test(start_paused = true)]
    async fn keep_the_time_of_a_status_event_through_a_late_snapshot_of_the_status_before() {
        let snapshots = vec![watched(7, "working"), watched(8, "waiting")];
        let lines = [vec![status_event("waiting")], snapshots, vec![line(snapshots::APP)]];
        assert_eq!(agents_once(lines, 1).await, [agent(4, "waiting", Some(TIME)), agent(5, "working", None)]);
    }

    #[rstest]
    #[case::before_the_snapshot(1)]
    #[case::after_the_snapshot(0)]
    #[tokio::test(start_paused = true)]
    async fn have_no_time_once_some_events_were_dropped(#[case] late: usize) {
        let events = vec![status_event("waiting"), line(events::DROPPED)];
        let lines = [events, vec![watched(7, "waiting")], vec![line(snapshots::APP)]];
        assert_eq!(agents_once(lines, late).await, [agent(4, "waiting", None), agent(5, "working", None)]);
    }

    #[tokio::test(start_paused = true)]
    async fn have_no_time_for_a_status_event_of_another_status() {
        let lines = [vec![status_event("waiting")], vec![line(snapshots::WATCH)], vec![line(snapshots::APP)]];
        assert_eq!(agents_once(lines, 1).await, [agent(4, "working", None), agent(5, "working", None)]);
    }
}
