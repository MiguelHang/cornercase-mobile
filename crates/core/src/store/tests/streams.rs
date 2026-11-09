use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::contract::fixtures::REAL;
use crate::cli::events::Event;
use crate::cli::events::fixtures as events;
use crate::cli::status::fixtures as snapshots;
use crate::store::streams::{APP, EVENTS, STATUS};

const SECRET: &str = "c2VjcmV0";

pub(super) type Lines = [Vec<String>; 3];

pub(super) fn line(json: &str) -> String {
    serde_json::from_str::<Value>(json).expect("a fixture is JSON").to_string()
}

pub(super) fn fine() -> Lines {
    [vec![line(events::OPENED), line(events::EXITED)], vec![line(snapshots::WATCH)], vec![line(snapshots::APP)]]
}

pub(super) fn serving(lines: Lines) -> FakeTransport {
    let fake = FakeTransport::default().answer(CONTRACT, &[REAL], &[], 0);
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

fn connected() -> Vec<Connection> {
    vec![Connection::Connecting, Connection::Connected { route: route(LAN).address().clone() }]
}

pub(super) fn snapshot(boot: &str, revision: u64, version: &str) -> String {
    json!({"boot": boot, "revision": revision, "version": version}).to_string()
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
    #[case::older_of_the_same_boot(snapshot("a", 6, "second"), "first")]
    #[case::as_old_of_the_same_boot(snapshot("a", 7, "second"), "first")]
    #[case::newer_of_the_same_boot(snapshot("a", 8, "second"), "second")]
    #[tokio::test(start_paused = true)]
    async fn takes_a_snapshot_only_if_newer_in_its_boot(#[case] next: String, #[case] kept: &str) {
        let [events, _, app] = fine();
        let (store, changes) = started([events, vec![snapshot("a", 7, "first"), next], app]).await;
        assert_eq!(store.session().map(|session| session.version), Some(kept.to_owned()));
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
