use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::cli::events::fixtures::{
    BELL, CLIPBOARD, CLOSED, DROPPED, EXITED, NOTICE, NOTICE_MACHINE, NOTICE_SOUND, OPENED, OPENED_ACCOUNTS,
    OPENED_PARENT, RECAP_FAILED, RECAP_FINISHED, RECAP_STARTED, STATUS,
};

fn parsed(text: &str) -> Event {
    serde_json::from_str(text).expect("an event line")
}

fn whole(text: &str) -> Value {
    serde_json::from_str(text).expect("the fixture is JSON")
}

fn notice(token: &str, status: &str, message: &str, machine: Option<&str>) -> What {
    What::Notice {
        token: token.to_owned(),
        agent: "claude".to_owned(),
        status: status.to_owned(),
        place: "shop › default".to_owned(),
        message: message.to_owned(),
        panes: vec![4],
        machine: machine.map(str::to_owned),
    }
}

fn opened(kind: Kind, parent: Option<u64>) -> What {
    What::Opened { kind, parent }
}

mod known_events {
    use super::*;

    #[rstest]
    #[case::status(
        STATUS,
        What::Status { agent: "claude".into(), from: Some("working".into()), to: Some("waiting".into()) }
    )]
    #[case::opened(OPENED, opened(Kind::Tab, None))]
    #[case::opened_with_a_parent(OPENED_PARENT, opened(Kind::Pane, Some(2)))]
    #[case::opened_with_accounts(OPENED_ACCOUNTS, opened(Kind::Pane, Some(2)))]
    #[case::closed(CLOSED, What::Closed { kind: Kind::Pane })]
    #[case::exited(EXITED, What::Exited { program: "cargo".into() })]
    #[case::notice(NOTICE, notice("cc-ab12cd-3", "waiting", "claude needs you in shop › default", None))]
    #[case::notice_with_a_sound(NOTICE_SOUND, notice("cc-ab12cd-4", "done", "claude finished in shop › default", None))]
    #[case::notice_from_another_machine(
        NOTICE_MACHINE,
        notice("cc-ab12cd-5", "waiting", "claude on vps needs you in shop › default", Some("vps"))
    )]
    #[case::dropped(DROPPED, What::Dropped { count: 12 })]
    fn reads_its_variant_and_fields(#[case] text: &str, #[case] what: What) {
        assert_eq!(parsed(text).what, what);
    }

    #[rstest]
    #[case::a_pane(STATUS, (Some(1), Some(2), Some(3), Some(4)))]
    #[case::a_tab(OPENED, (Some(1), Some(2), Some(3), None))]
    #[case::a_workspace(NOTICE, (Some(1), Some(2), None, None))]
    #[case::nothing(DROPPED, (None, None, None, None))]
    fn reads_the_ids_it_names(#[case] text: &str, #[case] ids: (Option<u64>, Option<u64>, Option<u64>, Option<u64>)) {
        let event = parsed(text);
        assert_eq!((event.project, event.workspace, event.tab, event.pane), ids);
        assert_eq!(event.time, "2026-10-16T09:30:00.123Z");
    }

    #[test]
    fn reads_a_status_without_a_previous_state() {
        let line = json!({"time": "t", "event": "status", "agent": "claude", "from": null, "to": "working"});
        let expected = What::Status { agent: "claude".into(), from: None, to: Some("working".into()) };
        assert_eq!(parsed(&line.to_string()).what, expected);
    }
}

mod other_events {
    use super::*;

    #[rstest]
    #[case::bell(BELL)]
    #[case::clipboard(CLIPBOARD)]
    #[case::recap_started(RECAP_STARTED)]
    #[case::recap_finished(RECAP_FINISHED)]
    #[case::recap_failed(RECAP_FAILED)]
    fn keep_an_unknown_event_whole(#[case] text: &str) {
        assert_eq!(parsed(text).what, What::Other(whole(text)));
    }

    #[rstest]
    #[case::status_without_an_agent(json!({"event": "status", "from": "working", "to": "waiting", "pane": 4}))]
    #[case::status_with_a_number(json!({"event": "status", "agent": 3, "from": null, "to": null, "pane": 4}))]
    #[case::opened_a_new_kind(json!({"event": "opened", "kind": "window", "project": 1}))]
    #[case::dropped_without_a_count(json!({"event": "dropped"}))]
    #[case::no_event_name(json!({"time": "t", "pane": 4}))]
    #[case::not_an_object(json!([1, 2]))]
    fn keep_an_unexpected_shape_whole(#[case] line: Value) {
        assert_eq!(parsed(&line.to_string()).what, What::Other(line));
    }

    #[test]
    fn keep_the_ids_of_an_unknown_event() {
        let event = parsed(BELL);
        assert_eq!((event.project, event.workspace, event.tab, event.pane), (Some(1), Some(2), Some(3), Some(4)));
    }

    #[test]
    fn keep_the_ids_beside_a_time_that_is_not_text() {
        let event =
            parsed(&json!({"time": 5, "event": "bell", "project": 1, "workspace": 2, "tab": 3, "pane": 4}).to_string());
        assert_eq!((event.project, event.workspace, event.tab, event.pane), (Some(1), Some(2), Some(3), Some(4)));
        assert_eq!(event.time, "");
    }

    #[test]
    fn fail_only_on_a_line_that_is_not_json() {
        assert!(serde_json::from_str::<Event>("{\"event\": \"status\"").is_err());
    }
}
